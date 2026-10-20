use super::sexpr::{self, Sexp};
use super::shape::{LayeredShape, layered_shape};
use super::units::Units;
use crate::BoardError;
use std::collections::HashMap;
use std::fmt::Write;
use traxide_contracts::geometry::{Point, Shape};
use traxide_contracts::{
    Design, LayerId, LayerShape, NetId, Routes, Track, Via, ViaKind, ViaKindId,
};

/// Specctra's y axis points up; the design's points down.
fn to_specctra(p: Point) -> Point {
    Point::new(p.x, -p.y)
}

/// A session with `routes`, in the resolution of `design` so that KiCad reads
/// back exactly what a router on that grid produced.
pub fn write_ses(name: &str, design: &Design, routes: &Routes) -> String {
    let units = Units::for_resolution(design.resolution);
    let layer = |id: LayerId| quoted(&design.layers[id.index()].name);
    let xy = |p: Point| {
        let p = to_specctra(p);
        format!("{} {}", units.steps(p.x), units.steps(p.y))
    };
    let mut ses = String::new();
    // Writing to a String cannot fail.
    let mut line = |text: String| {
        let _ = writeln!(ses, "{text}");
    };
    line(format!("(session {}", quoted(name)));
    line("  (routes".into());
    line(format!("    {}", units.resolution_text()));
    line(format!(
        "    (parser (host_cad traxide) (host_version {}))",
        env!("CARGO_PKG_VERSION")
    ));
    line("    (library_out".into());
    let mut used_kinds: Vec<ViaKindId> = routes.vias.iter().map(|v| v.kind).collect();
    used_kinds.sort();
    used_kinds.dedup();
    for kind in used_kinds.into_iter().map(|k| design.via_kind(k)) {
        line(format!("      (padstack {}", quoted(&kind.name)));
        for LayerShape { layer: on, shape } in &kind.copper {
            let shape = match shape {
                Shape::Capsule { segment, radius } if segment.a == segment.b => {
                    format!(
                        "circle {} {} {}",
                        layer(*on),
                        units.steps(2 * radius),
                        xy(segment.a)
                    )
                }
                Shape::Capsule { segment, radius } => {
                    format!(
                        "path {} {} {} {}",
                        layer(*on),
                        units.steps(2 * radius),
                        xy(segment.a),
                        xy(segment.b)
                    )
                }
                Shape::Polygon(polygon) => {
                    let points: Vec<String> = polygon.points().iter().map(|&p| xy(p)).collect();
                    format!("polygon {} 0 {}", layer(*on), points.join(" "))
                }
            };
            line(format!("        (shape ({shape}))"));
        }
        line("        (attach off)".into());
        line("      )".into());
    }
    line("    )".into());
    line("    (network_out".into());
    for (index, net) in design.nets.iter().enumerate() {
        let id = NetId::from_index(index);
        let tracks: Vec<&Track> = routes.tracks.iter().filter(|t| t.net == id).collect();
        let vias: Vec<&Via> = routes.vias.iter().filter(|v| v.net == id).collect();
        if tracks.is_empty() && vias.is_empty() {
            continue;
        }
        line(format!("      (net {}", quoted(&net.name)));
        for track in tracks {
            let path: Vec<String> = track.path.iter().map(|&p| xy(p)).collect();
            line(format!(
                "        (wire (path {} {} {}))",
                layer(track.layer),
                units.steps(track.width),
                path.join(" ")
            ));
        }
        for via in vias {
            line(format!(
                "        (via {} {})",
                quoted(&design.via_kind(via.kind).name),
                xy(via.position)
            ));
        }
        line("      )".into());
    }
    line("    )".into());
    line("  )".into());
    line(")".into());
    ses
}

/// The routes of a session, matched to the nets, layers and vias of `design`.
pub fn parse_ses(text: &str, design: &Design) -> Result<Routes, BoardError> {
    let root = sexpr::parse(text)?;
    if root.keyword() != Some("session") {
        return Err(root.invalid("not a Specctra session: expected (session …)"));
    }
    let session_routes = root.required_child("routes")?;
    let units = Units::of_session(session_routes.required_child("resolution")?)?;
    let layer_named = |sexp: &Sexp| -> Result<LayerId, BoardError> {
        let name = sexp
            .atom()
            .ok_or_else(|| sexp.invalid("expected a layer"))?;
        design
            .layers
            .iter()
            .position(|l| l.name == name)
            .map(LayerId::from_index)
            .ok_or_else(|| sexp.invalid(format!("unknown layer {name}")))
    };
    let defined_padstacks = match session_routes.child("library_out") {
        Some(library) => padstacks(library, &units, &layer_named)?,
        None => HashMap::new(),
    };
    // A session rounds every length to its resolution.
    let same_within = units.resolution_nm();
    let mut routes = Routes::default();
    let Some(network) = session_routes.child("network_out") else {
        return Ok(routes);
    };
    for net_out in network.children("net") {
        let name = net_out.arg_atom(0, "net name")?;
        let net = design
            .nets
            .iter()
            .position(|n| n.name == name)
            .map(NetId::from_index)
            .ok_or_else(|| net_out.invalid(format!("unknown net {name}")))?;
        for wire in net_out.children("wire") {
            let path = wire.required_child("path")?;
            let args = path.args();
            let (Some(on), Some(width)) = (args.first(), args.get(1)) else {
                return Err(path.invalid("expected (path layer width x y …)"));
            };
            let points = units
                .points(&args[2..])?
                .into_iter()
                .map(to_specctra)
                .collect();
            routes.tracks.push(Track {
                net,
                layer: layer_named(on)?,
                width: units.length(width)?,
                path: points,
            });
        }
        for via in net_out.children("via") {
            let args = via.args();
            let (Some(kind), Some(x), Some(y)) =
                (args.first().and_then(Sexp::atom), args.get(1), args.get(2))
            else {
                return Err(via.invalid("expected (via padstack x y)"));
            };
            let defined_copper = defined_padstacks.get(kind).map(Vec::as_slice);
            let kind = via_kind(design, kind, defined_copper, same_within).ok_or_else(|| {
                match defined_copper {
                    Some(_) => via.invalid(format!(
                        "via padstack {kind} has copper that no via of the design has"
                    )),
                    None => via.invalid(format!("unknown via padstack {kind}")),
                }
            })?;
            routes.vias.push(Via {
                net,
                kind,
                position: to_specctra(units.point(x, y)?),
            });
        }
    }
    Ok(routes)
}

/// The copper of the padstacks a session defines, by name, relative to their
/// centre.
fn padstacks<'s>(
    library_out: &'s Sexp,
    units: &Units,
    layer_named: &impl Fn(&Sexp) -> Result<LayerId, BoardError>,
) -> Result<HashMap<&'s str, Vec<LayerShape>>, BoardError> {
    library_out
        .children("padstack")
        .map(|padstack| {
            let name = padstack.arg_atom(0, "padstack name")?;
            let mut copper = Vec::new();
            for shape in padstack.children("shape").filter_map(|s| s.args().first()) {
                let LayeredShape { layers, shape } =
                    layered_shape(shape, units, |on| layer_named(on).map(|id| vec![id]))?;
                let shape = shape.map(to_specctra);
                copper.extend(layers.into_iter().map(|layer| LayerShape {
                    layer,
                    shape: shape.clone(),
                }));
            }
            Ok((name, copper))
        })
        .collect()
}

/// The design's via a session's via padstack stands for. Routers define the
/// padstacks of the vias they place in the session, under names of their own
/// (Freerouting 2.5 shortens `Round1$13.779528` to `Round1$13`): such a
/// padstack is the via with the same copper, preferably of the same name. A
/// padstack the session does not define is the via of that name.
fn via_kind(
    design: &Design,
    name: &str,
    defined_copper: Option<&[LayerShape]>,
    same_within: i32,
) -> Option<ViaKindId> {
    let kinds = &design.via_kinds;
    let index = match defined_copper {
        None => kinds.iter().position(|k| k.name == name),
        Some(copper) => {
            let same = |kind: &ViaKind| same_copper(&kind.copper, copper, same_within);
            kinds
                .iter()
                .position(|k| k.name == name && same(k))
                .or_else(|| kinds.iter().position(same))
        }
    };
    index.map(ViaKindId::from_index)
}

/// Whether two padstacks have the same shapes on the same layers, telling
/// apart only lengths more than `same_within` apart.
fn same_copper(a: &[LayerShape], b: &[LayerShape], same_within: i32) -> bool {
    let has_all_of = |these: &[LayerShape], those: &[LayerShape]| {
        those.iter().all(|that| {
            these.iter().any(|this| {
                this.layer == that.layer && same_shape(&this.shape, &that.shape, same_within)
            })
        })
    };
    a.len() == b.len() && has_all_of(a, b) && has_all_of(b, a)
}

fn same_shape(a: &Shape, b: &Shape, same_within: i32) -> bool {
    let close = |p: i32, q: i32| p.abs_diff(q) <= same_within.unsigned_abs();
    let near = |p: Point, q: Point| close(p.x, q.x) && close(p.y, q.y);
    match (a, b) {
        (
            Shape::Capsule { segment, radius },
            Shape::Capsule {
                segment: other,
                radius: other_radius,
            },
        ) => near(segment.a, other.a) && near(segment.b, other.b) && close(*radius, *other_radius),
        (Shape::Polygon(polygon), Shape::Polygon(other)) => {
            polygon.points().len() == other.points().len()
                && polygon
                    .points()
                    .iter()
                    .zip(other.points())
                    .all(|(&p, &q)| near(p, q))
        }
        _ => false,
    }
}

/// Names with characters beyond these would end a bare token or confuse readers.
fn quoted(name: &str) -> String {
    let plain = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.+/".contains(c));
    if plain {
        name.to_owned()
    } else {
        format!("\"{name}\"")
    }
}
