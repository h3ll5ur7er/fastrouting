use super::sexpr::{self, Sexp};
use super::units::Units;
use crate::BoardError;
use std::fmt::Write;
use traxide_contracts::geometry::{Point, Shape};
use traxide_contracts::{Design, LayerId, LayerShape, NetId, Routes, Track, Via, ViaKindId};

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
            let kind = design
                .via_kinds
                .iter()
                .position(|k| k.name == kind)
                .map(ViaKindId::from_index)
                .ok_or_else(|| via.invalid(format!("unknown via padstack {kind}")))?;
            routes.vias.push(Via {
                net,
                kind,
                position: to_specctra(units.point(x, y)?),
            });
        }
    }
    Ok(routes)
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
