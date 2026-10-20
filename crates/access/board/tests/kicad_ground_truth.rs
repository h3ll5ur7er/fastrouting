//! KiCad's own placement of every pad, exported by e2e/kicad/fixtures.py from
//! KiCad 10, is the ground truth for how a DSN file and a board file position
//! pads.

use serde::Deserialize;
use std::path::Path;
use traxide_board_access::load_design;
use traxide_contracts::Design;

#[derive(Debug, Deserialize)]
struct KicadPad {
    name: String,
    net: Option<String>,
    x: i32,
    y: i32,
    layers: Vec<String>,
    bounds: [i32; 4],
}

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/kicad10/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn pads_land_where_kicad_places_them() {
    let design = load_design(Path::new(&fixture("placements.dsn"))).expect("KiCad's DSN is valid");
    // DSN rounds KiCad's nanometres to its resolution; rotated shapes add rounding of their own.
    let mismatches = mismatches(&design, design.resolution);
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn pads_of_a_board_file_land_where_kicad_places_them() {
    let design =
        load_design(Path::new(&fixture("placements.kicad_pcb"))).expect("KiCad's board is valid");
    // Rotations round to the nanometre, in KiCad as here.
    let mismatches = mismatches(&design, 1);
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

fn mismatches(design: &Design, tolerance: i32) -> Vec<String> {
    let truth: Vec<KicadPad> = serde_json::from_str(
        &std::fs::read_to_string(fixture("placements.pads.json")).expect("fixture exists"),
    )
    .expect("valid JSON");
    let mut mismatches = Vec::new();
    for expected in &truth {
        let Some(pad) = design.pads.iter().find(|p| p.name == expected.name) else {
            mismatches.push(format!("{}: missing", expected.name));
            continue;
        };
        let near = |a: i32, b: i32| (a - b).abs() <= tolerance;
        if !near(pad.position.x, expected.x) || !near(pad.position.y, expected.y) {
            mismatches.push(format!(
                "{}: at {:?}, KiCad has ({}, {})",
                expected.name, pad.position, expected.x, expected.y
            ));
        }
        let layers: Vec<&str> = pad
            .copper
            .iter()
            .map(|c| design.layers[c.layer.index()].name.as_str())
            .collect();
        if layers != expected.layers {
            mismatches.push(format!(
                "{}: on {layers:?}, KiCad has {:?}",
                expected.name, expected.layers
            ));
        }
        let bounds = pad.copper[0].shape.bounds();
        let actual = [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y];
        if actual
            .iter()
            .zip(expected.bounds)
            .any(|(&a, b)| !near(a, b))
        {
            mismatches.push(format!(
                "{}: bounds {actual:?}, KiCad has {:?}",
                expected.name, expected.bounds
            ));
        }
        let net = pad.net.map(|n| design.net(n).name.as_str());
        if net != expected.net.as_deref() {
            mismatches.push(format!(
                "{}: net {net:?}, KiCad has {:?}",
                expected.name, expected.net
            ));
        }
    }
    mismatches
}
