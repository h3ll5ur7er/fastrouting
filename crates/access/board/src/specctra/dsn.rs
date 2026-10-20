use super::sexpr::{self, Sexp};
use super::units::Units;
use crate::BoardError;
use std::collections::HashMap;
use traxide_contracts::geometry::{Point, Polygon, Rect, Segment, Shape, Transform};
use traxide_contracts::{
    Design, DesignRules, Keepout, KeepoutScope, Layer, LayerId, LayerShape, Net, NetClass,
    NetClassId, NetId, Pad, PadId, Routes, Track, Via, ViaKind, ViaKindId,
};

pub fn parse_dsn(text: &str) -> Result<Design, BoardError> {
    let root = sexpr::parse(text)?;
    if root.keyword() != Some("pcb") {
        return Err(root.invalid("not a Specctra design: expected (pcb …)"));
    }
    DsnReader::new(&root)?.read()
}

/// Specctra's y axis points up; the design's, like KiCad's, points down.
fn to_board(p: Point) -> Point {
    Point::new(p.x, -p.y)
}

/// A shape on the layers it covers, in Specctra's coordinates.
struct LayeredShape {
    layers: Vec<LayerId>,
    shape: Shape,
}

struct Image<'a> {
    pins: Vec<Pin<'a>>,
    keepouts: Vec<(KeepoutScope, LayeredShape)>,
}

struct Pin<'a> {
    padstack: &'a str,
    id: &'a str,
    at: Point,
    rotation_degrees: f64,
}

struct DsnReader<'a> {
    root: &'a Sexp,
    structure: &'a Sexp,
    units: Units,
    resolution_nm: i32,
    layers: Vec<Layer>,
    layer_by_name: HashMap<&'a str, LayerId>,
}

impl<'a> DsnReader<'a> {
    fn new(root: &'a Sexp) -> Result<Self, BoardError> {
        let structure = root.required_child("structure")?;
        let mut layers = Vec::new();
        let mut layer_by_name = HashMap::new();
        for layer in structure.children("layer") {
            let name = layer.arg_atom(0, "layer name")?;
            let kind = layer.child("type").and_then(|t| t.atoms().next());
            layer_by_name.insert(name, LayerId::from_index(layers.len()));
            layers.push(Layer {
                name: name.to_owned(),
                routable: kind != Some("power"),
            });
        }
        if layers.is_empty() {
            return Err(structure.invalid("the design has no layers"));
        }
        let units = Units::of_design(root)?;
        Ok(Self {
            root,
            structure,
            units: units.design_lengths(),
            resolution_nm: units.resolution_nm(),
            layers,
            layer_by_name,
        })
    }

    fn read(self) -> Result<Design, BoardError> {
        let padstacks = self.padstacks()?;
        let (mut pads, footprint_keepouts) = self.placed(&padstacks)?;
        let pad_by_name: HashMap<String, PadId> = pads
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.clone(), PadId::from_index(i)))
            .collect();
        let mut via_kinds = ViaKinds::default();
        let rules = self.rules(&padstacks, &mut via_kinds)?;
        let mut nets = self.nets(&pad_by_name)?;
        for (index, net) in nets.iter_mut().enumerate() {
            net.class = rules
                .class_of_net
                .get(net.name.as_str())
                .copied()
                .unwrap_or(rules.design.default_class);
            for &pad in &net.pads {
                pads[pad.index()].net = Some(NetId::from_index(index));
            }
        }
        let existing = self.wiring(&nets, &padstacks, &mut via_kinds)?;
        Ok(Design {
            outline: self.outline()?,
            keepouts: [self.keepouts()?, footprint_keepouts].concat(),
            layers: self.layers,
            nets,
            pads,
            via_kinds: via_kinds.kinds,
            existing,
            rules: rules.design,
            resolution: self.resolution_nm,
        })
    }

    fn layers_named(&self, name: &Sexp) -> Result<Vec<LayerId>, BoardError> {
        let text = name
            .atom()
            .ok_or_else(|| name.invalid("expected a layer name"))?;
        match text {
            "signal" | "pcb" | "*" => Ok((0..self.layers.len()).map(LayerId::from_index).collect()),
            _ => self
                .layer_by_name
                .get(text)
                .map(|&id| vec![id])
                .ok_or_else(|| name.invalid(format!("unknown layer {text}"))),
        }
    }

    fn shape(&self, sexp: &Sexp) -> Result<LayeredShape, BoardError> {
        let args = sexp.args();
        let layer = args.first().ok_or_else(|| sexp.invalid("missing layer"))?;
        let number = |i: usize| args.get(i).ok_or_else(|| sexp.invalid("missing number"));
        let shape = match sexp.keyword() {
            Some("circle") => {
                let center = match (args.get(2), args.get(3)) {
                    (Some(x), Some(y)) => self.units.point(x, y)?,
                    _ => Point::default(),
                };
                Shape::circle(center, self.units.length(number(1)?)?)
            }
            Some("rect") => {
                let corners = self.units.points(&args[1..])?;
                Shape::Polygon(Polygon::rectangle(
                    Rect::around(corners).ok_or_else(|| sexp.invalid("empty rect"))?,
                ))
            }
            Some("polygon") => Shape::Polygon(self.polygon(sexp, &args[2..])?),
            Some("path") => {
                let width = self.units.length(number(1)?)?;
                match self.units.points(&args[2..])?.as_slice() {
                    [a] => Shape::circle(*a, width),
                    [a, b] => Shape::track(Segment::new(*a, *b), width),
                    _ => {
                        return Err(sexp
                            .invalid("paths of more than two points are not supported as shapes"));
                    }
                }
            }
            other => {
                return Err(sexp.invalid(format!("unsupported shape {}", other.unwrap_or("?"))));
            }
        };
        Ok(LayeredShape {
            layers: self.layers_named(layer)?,
            shape,
        })
    }

    fn polygon(&self, sexp: &Sexp, coordinates: &[Sexp]) -> Result<Polygon, BoardError> {
        let mut points = self.units.points(coordinates)?;
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        Polygon::new(points).ok_or_else(|| sexp.invalid("a polygon needs at least three points"))
    }

    fn outline(&self) -> Result<Polygon, BoardError> {
        let boundary = self.structure.required_child("boundary")?;
        let shape = boundary
            .args()
            .first()
            .ok_or_else(|| boundary.invalid("empty boundary"))?;
        let polygon = match shape.keyword() {
            Some("path" | "polygon") => {
                self.polygon(shape, shape.args().get(2..).unwrap_or_default())?
            }
            Some("rect") => {
                let corners = self.units.points(&shape.args()[1..])?;
                Polygon::rectangle(
                    Rect::around(corners).ok_or_else(|| shape.invalid("empty rect"))?,
                )
            }
            _ => return Err(shape.invalid("unsupported boundary shape")),
        };
        Ok(polygon.map(to_board))
    }

    fn keepouts(&self) -> Result<Vec<Keepout>, BoardError> {
        let keepouts = self.keepouts_in(self.structure)?;
        Ok(keepouts
            .into_iter()
            .flat_map(|(scope, LayeredShape { layers, shape })| {
                self.keepouts_on(&layers, shape.map(to_board), scope)
            })
            .collect())
    }

    /// The keepouts listed in `parent`, the structure or an image, as Specctra states them.
    fn keepouts_in(&self, parent: &Sexp) -> Result<Vec<(KeepoutScope, LayeredShape)>, BoardError> {
        let scopes = [
            ("keepout", KeepoutScope::Everything),
            ("wire_keepout", KeepoutScope::TracksOnly),
            ("via_keepout", KeepoutScope::ViasOnly),
        ];
        let mut keepouts = Vec::new();
        for (keyword, scope) in scopes {
            for keepout in parent.children(keyword) {
                let shape = keepout
                    .args()
                    .iter()
                    .find(|a| matches!(a.keyword(), Some("circle" | "rect" | "polygon" | "path")))
                    .ok_or_else(|| keepout.invalid("keepout without a shape"))?;
                keepouts.push((scope, self.shape(shape)?));
            }
        }
        Ok(keepouts)
    }

    /// One keepout for all layers, or one per layer it covers.
    fn keepouts_on(&self, layers: &[LayerId], shape: Shape, scope: KeepoutScope) -> Vec<Keepout> {
        if layers.len() == self.layers.len() {
            return vec![Keepout {
                layer: None,
                shape,
                scope,
            }];
        }
        layers
            .iter()
            .map(|&layer| Keepout {
                layer: Some(layer),
                shape: shape.clone(),
                scope,
            })
            .collect()
    }

    fn padstacks(&self) -> Result<HashMap<&'a str, Vec<LayeredShape>>, BoardError> {
        let library = self.root.required_child("library")?;
        library
            .children("padstack")
            .map(|padstack| {
                let name = padstack.arg_atom(0, "padstack name")?;
                let shapes = padstack
                    .children("shape")
                    .filter_map(|s| s.args().first())
                    .map(|s| self.shape(s))
                    .collect::<Result<_, _>>()?;
                Ok((name, shapes))
            })
            .collect()
    }

    fn images(&self) -> Result<HashMap<&'a str, Image<'a>>, BoardError> {
        let library = self.root.required_child("library")?;
        library
            .children("image")
            .map(|image| {
                let name = image.arg_atom(0, "image name")?;
                let pins = image
                    .children("pin")
                    .map(|pin| self.pin(pin))
                    .collect::<Result<_, _>>()?;
                Ok((
                    name,
                    Image {
                        pins,
                        keepouts: self.keepouts_in(image)?,
                    },
                ))
            })
            .collect()
    }

    /// `(pin padstack [(rotate degrees)] id x y)`
    fn pin(&self, pin: &'a Sexp) -> Result<Pin<'a>, BoardError> {
        let atoms: Vec<&Sexp> = pin.args().iter().filter(|a| a.atom().is_some()).collect();
        let [padstack, id, x, y] = atoms[..] else {
            return Err(pin.invalid("expected (pin padstack id x y)"));
        };
        let rotation_degrees = match pin.child("rotate") {
            Some(rotate) => degrees(rotate, rotate.arg_atom(0, "angle")?)?,
            None => 0.0,
        };
        Ok(Pin {
            padstack: padstack.atom().unwrap_or_default(),
            id: id.atom().unwrap_or_default(),
            at: self.units.point(x, y)?,
            rotation_degrees,
        })
    }

    /// The pads and keepouts of every placed component.
    fn placed(
        &self,
        padstacks: &HashMap<&str, Vec<LayeredShape>>,
    ) -> Result<(Vec<Pad>, Vec<Keepout>), BoardError> {
        let images = self.images()?;
        let last_layer = self.layers.len() - 1;
        let mut pads = Vec::new();
        let mut keepouts = Vec::new();
        for component in self.root.required_child("placement")?.children("component") {
            let image_name = component.arg_atom(0, "image name")?;
            let image = images
                .get(image_name)
                .ok_or_else(|| component.invalid(format!("unknown image {image_name}")))?;
            for place in component.children("place") {
                let reference = place.arg_atom(0, "reference")?;
                let args = place.args();
                let (Some(x), Some(y)) = (args.get(1), args.get(2)) else {
                    return Err(place.invalid("place without a position"));
                };
                let back = place.arg_atom(3, "side").is_ok_and(|side| side == "back");
                let rotation_degrees = match args.get(4).and_then(Sexp::atom) {
                    Some(angle) => degrees(place, angle)?,
                    None => 0.0,
                };
                let placement = Transform {
                    origin: self.units.point(x, y)?,
                    rotation_degrees,
                    mirror_x: back,
                };
                // Placing a component on the back mirrors its layer stack too.
                let layer_on_side = |l: LayerId| {
                    if back {
                        LayerId::from_index(last_layer - l.index())
                    } else {
                        l
                    }
                };
                for (scope, LayeredShape { layers, shape }) in &image.keepouts {
                    let layers: Vec<LayerId> = layers.iter().map(|&l| layer_on_side(l)).collect();
                    let shape = shape.map(|p| to_board(placement.apply(p)));
                    keepouts.extend(self.keepouts_on(&layers, shape, *scope));
                }
                for pin in &image.pins {
                    let shapes = padstacks.get(pin.padstack).ok_or_else(|| {
                        place.invalid(format!("unknown padstack {}", pin.padstack))
                    })?;
                    let local = Transform {
                        origin: pin.at,
                        rotation_degrees: pin.rotation_degrees,
                        mirror_x: false,
                    };
                    let to_absolute = local.then(placement);
                    let copper = shapes
                        .iter()
                        .flat_map(|s| s.layers.iter().map(move |&l| (l, &s.shape)))
                        .map(|(layer, shape)| LayerShape {
                            layer: layer_on_side(layer),
                            shape: shape.map(|p| to_board(to_absolute(p))),
                        })
                        .collect();
                    pads.push(Pad {
                        name: format!("{reference}-{}", pin.id),
                        net: None,
                        position: to_board(placement.apply(pin.at)),
                        copper,
                    });
                }
            }
        }
        Ok((pads, keepouts))
    }

    fn rules(
        &self,
        padstacks: &HashMap<&str, Vec<LayeredShape>>,
        via_kinds: &mut ViaKinds,
    ) -> Result<CompiledClasses<'a>, BoardError> {
        let board_rule = self.structure.required_child("rule")?;
        let (width, clearance) = self.rule(board_rule, None)?;
        let mut default_vias = Vec::new();
        for via in self.structure.children("via") {
            for name in via.args() {
                default_vias.push(via_kinds.named(name, padstacks)?);
            }
        }
        let mut classes = vec![NetClass {
            name: "default".into(),
            track_width: width,
            clearance,
            via_kinds: default_vias.clone(),
        }];
        let mut class_of_net = HashMap::new();
        for class in self.root.required_child("network")?.children("class") {
            let id = NetClassId::from_index(classes.len());
            let name = class.arg_atom(0, "class name")?;
            let (track_width, clearance) = match class.child("rule") {
                Some(rule) => self.rule(rule, Some((width, clearance)))?,
                None => (width, clearance),
            };
            let mut vias = Vec::new();
            for use_via in class
                .children("circuit")
                .flat_map(|c| c.children("use_via"))
            {
                for name in use_via.args() {
                    vias.push(via_kinds.named(name, padstacks)?);
                }
            }
            for net in class.atoms().skip(1) {
                class_of_net.insert(net, id);
            }
            classes.push(NetClass {
                name: name.to_owned(),
                track_width,
                clearance,
                via_kinds: if vias.is_empty() {
                    default_vias.clone()
                } else {
                    vias
                },
            });
        }
        Ok(CompiledClasses {
            design: DesignRules {
                classes,
                default_class: NetClassId(0),
                edge_clearance: self.edge_clearance(clearance),
            },
            class_of_net,
        })
    }

    /// DSN cannot state how far copper must stay from the board edge. KiCad's DRC
    /// applies its board setup, which is 0.5 mm unless the designer changed it,
    /// so boards from KiCad keep at least that; others the board-wide clearance.
    fn edge_clearance(&self, clearance: i32) -> i32 {
        const KICAD_DEFAULT_EDGE_CLEARANCE: i32 = 500_000;
        let from_kicad = self
            .root
            .child("parser")
            .and_then(|parser| parser.child("host_cad"))
            .and_then(|host| host.atoms().next())
            .is_some_and(|host| host.contains("KiCad"));
        if from_kicad {
            clearance.max(KICAD_DEFAULT_EDGE_CLEARANCE)
        } else {
            clearance
        }
    }

    /// The width and the general clearance of a `(rule …)`; clearances limited to
    /// a `(type …)` of object refine pad-to-pad spacing, which routing never sets.
    fn rule(&self, rule: &Sexp, defaults: Option<(i32, i32)>) -> Result<(i32, i32), BoardError> {
        let length = |keyword: &str, default: Option<i32>| -> Result<i32, BoardError> {
            let general = rule.children(keyword).find(|c| c.child("type").is_none());
            match (general.and_then(|c| c.args().first()), default) {
                (Some(value), _) => self.units.length(value),
                (None, Some(default)) => Ok(default),
                (None, None) => Err(rule.invalid(format!("rule without a {keyword}"))),
            }
        };
        Ok((
            length("width", defaults.map(|d| d.0))?,
            length("clearance", defaults.map(|d| d.1))?,
        ))
    }

    fn nets(&self, pad_by_name: &HashMap<String, PadId>) -> Result<Vec<Net>, BoardError> {
        self.root
            .required_child("network")?
            .children("net")
            .map(|net| {
                let name = net.arg_atom(0, "net name")?;
                let pads = net
                    .children("pins")
                    .flat_map(|pins| pins.args())
                    .map(|pin| {
                        let pin_name = pin.atom().ok_or_else(|| pin.invalid("expected a pin"))?;
                        pad_by_name
                            .get(pin_name)
                            .copied()
                            .ok_or_else(|| pin.invalid(format!("unknown pin {pin_name}")))
                    })
                    .collect::<Result<_, _>>()?;
                Ok(Net {
                    name: name.to_owned(),
                    class: NetClassId(0),
                    pads,
                })
            })
            .collect()
    }

    fn wiring(
        &self,
        nets: &[Net],
        padstacks: &HashMap<&str, Vec<LayeredShape>>,
        via_kinds: &mut ViaKinds,
    ) -> Result<Routes, BoardError> {
        let mut routes = Routes::default();
        let Some(wiring) = self.root.child("wiring") else {
            return Ok(routes);
        };
        let net_of = |item: &Sexp| -> Result<NetId, BoardError> {
            let name = item.required_child("net")?.arg_atom(0, "net name")?;
            nets.iter()
                .position(|n| n.name == name)
                .map(NetId::from_index)
                .ok_or_else(|| item.invalid(format!("unknown net {name}")))
        };
        for wire in wiring.children("wire") {
            let path = wire.required_child("path")?;
            let layer = self.layers_named(
                path.args()
                    .first()
                    .ok_or_else(|| path.invalid("missing layer"))?,
            )?;
            routes.tracks.push(Track {
                net: net_of(wire)?,
                layer: layer[0],
                width: self.units.length(
                    path.args()
                        .get(1)
                        .ok_or_else(|| path.invalid("missing width"))?,
                )?,
                path: self
                    .units
                    .points(path.args().get(2..).unwrap_or_default())?
                    .into_iter()
                    .map(to_board)
                    .collect(),
            });
        }
        for via in wiring.children("via") {
            let args = via.args();
            let (Some(name), Some(x), Some(y)) = (args.first(), args.get(1), args.get(2)) else {
                return Err(via.invalid("expected (via padstack x y)"));
            };
            routes.vias.push(Via {
                net: net_of(via)?,
                kind: via_kinds.named(name, padstacks)?,
                position: to_board(self.units.point(x, y)?),
            });
        }
        Ok(routes)
    }
}

struct CompiledClasses<'a> {
    design: DesignRules,
    class_of_net: HashMap<&'a str, NetClassId>,
}

#[derive(Default)]
struct ViaKinds {
    kinds: Vec<ViaKind>,
}

impl ViaKinds {
    fn named(
        &mut self,
        name: &Sexp,
        padstacks: &HashMap<&str, Vec<LayeredShape>>,
    ) -> Result<ViaKindId, BoardError> {
        let text = name
            .atom()
            .ok_or_else(|| name.invalid("expected a via padstack"))?;
        if let Some(index) = self.kinds.iter().position(|k| k.name == text) {
            return Ok(ViaKindId::from_index(index));
        }
        let shapes = padstacks
            .get(text)
            .ok_or_else(|| name.invalid(format!("unknown via padstack {text}")))?;
        let copper = shapes
            .iter()
            .flat_map(|s| {
                s.layers.iter().map(move |&layer| LayerShape {
                    layer,
                    shape: s.shape.map(to_board),
                })
            })
            .collect();
        self.kinds.push(ViaKind {
            name: text.to_owned(),
            copper,
        });
        Ok(ViaKindId::from_index(self.kinds.len() - 1))
    }
}

fn degrees(at: &Sexp, text: &str) -> Result<f64, BoardError> {
    text.parse()
        .map_err(|_| at.invalid(format!("not an angle: {text}")))
}
