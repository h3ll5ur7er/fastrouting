use traxide_board_access::{BoardError, load_design, specctra};
use traxide_contracts::geometry::{Point, Rect, Shape};
use traxide_contracts::{Design, LayerId, Pad};

fn two_resistors() -> Design {
    load_design(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/two-resistors.dsn"
        )
        .as_ref(),
    )
    .expect("the fixture is valid")
}

fn mm(value: f64) -> i32 {
    (value * 1_000_000.0).round() as i32
}

fn pad<'a>(design: &'a Design, name: &str) -> &'a Pad {
    design
        .pads
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no pad {name}"))
}

fn layer(design: &Design, name: &str) -> LayerId {
    LayerId::from_index(
        design
            .layers
            .iter()
            .position(|l| l.name == name)
            .expect("layer exists"),
    )
}

#[test]
fn reads_copper_layers_from_top_to_bottom() {
    let names: Vec<_> = two_resistors().layers.into_iter().map(|l| l.name).collect();
    assert_eq!(names, ["F.Cu", "B.Cu"]);
}

#[test]
fn converts_to_nanometres_with_y_pointing_down_as_in_kicad() {
    let design = two_resistors();
    assert_eq!(pad(&design, "R1-1").position, Point::new(mm(4.0), mm(5.0)));
    assert_eq!(
        pad(&design, "R2-1").position,
        Point::new(mm(23.73), mm(15.0))
    );
}

#[test]
fn reads_the_coordinate_resolution() {
    assert_eq!(two_resistors().resolution, 100);
}

#[test]
fn smd_pads_have_copper_on_their_layer_only() {
    let design = two_resistors();
    let copper = &pad(&design, "R1-1").copper;
    assert_eq!(copper.len(), 1);
    assert_eq!(copper[0].layer, layer(&design, "F.Cu"));
    let expected = Rect {
        min: Point::new(mm(3.5), mm(4.4)),
        max: Point::new(mm(4.5), mm(5.6)),
    };
    assert_eq!(copper[0].shape.bounds(), expected);
}

#[test]
fn pins_rotate_their_pad_shapes() {
    let design = two_resistors();
    let expected = Rect {
        min: Point::new(mm(5.4), mm(4.5)),
        max: Point::new(mm(6.6), mm(5.5)),
    };
    assert_eq!(pad(&design, "R1-2").copper[0].shape.bounds(), expected);
}

#[test]
fn through_hole_pads_have_copper_on_every_layer() {
    let design = two_resistors();
    let copper = &pad(&design, "R2-2").copper;
    assert_eq!(copper.len(), 2);
    assert_eq!(
        copper[0].shape,
        Shape::circle(Point::new(mm(26.27), mm(15.0)), mm(1.8))
    );
}

#[test]
fn components_on_the_back_are_mirrored_and_their_pads_move_to_the_bottom_layer() {
    let design = two_resistors();
    let r3 = pad(&design, "R3-1");
    assert_eq!(r3.position, Point::new(mm(15.0), mm(9.0)));
    assert_eq!(r3.copper[0].layer, layer(&design, "B.Cu"));
}

#[test]
fn reads_nets_with_their_pads() {
    let design = two_resistors();
    let net = design
        .nets
        .iter()
        .find(|n| n.name == "Net with spaces")
        .expect("net exists");
    let pads: Vec<_> = net
        .pads
        .iter()
        .map(|&p| design.pad(p).name.as_str())
        .collect();
    assert_eq!(pads, ["R2-2", "R1-2", "R3-1"]);
    assert_eq!(
        pad(&design, "R3-1")
            .net
            .map(|n| design.net(n).name.as_str()),
        Some("Net with spaces")
    );
}

#[test]
fn reads_net_classes_with_their_rules() {
    let design = two_resistors();
    let net = design
        .nets
        .iter()
        .find(|n| n.name == "Net with spaces")
        .expect("net exists");
    let class = &design.rules.classes[net.class.index()];
    assert_eq!(
        (class.name.as_str(), class.track_width, class.clearance),
        ("Power", mm(0.5), mm(0.3))
    );
    assert_eq!(class.via_kinds.len(), 1);
}

#[test]
fn copper_of_no_net_follows_the_board_wide_rule() {
    let design = two_resistors();
    let default = &design.rules.classes[design.rules.default_class.index()];
    assert_eq!((default.track_width, default.clearance), (mm(0.2), 200_100));
}

#[test]
fn reads_via_padstacks() {
    let design = two_resistors();
    assert_eq!(design.via_kinds.len(), 1);
    let via = &design.via_kinds[0];
    assert_eq!(via.name, "Via[0-1]_600:300_um");
    assert_eq!(via.copper.len(), 2);
    assert_eq!(
        via.copper[0].shape,
        Shape::circle(Point::new(0, 0), mm(0.6))
    );
}

#[test]
fn reads_the_board_outline() {
    let design = two_resistors();
    let expected = Rect {
        min: Point::new(0, 0),
        max: Point::new(mm(30.0), mm(20.0)),
    };
    assert_eq!(design.outline.bounds(), expected);
    assert_eq!(design.outline.points().len(), 4);
}

#[test]
fn reads_existing_tracks() {
    let design = two_resistors();
    let track = &design.existing.tracks[0];
    assert_eq!(design.net(track.net).name, "A");
    assert_eq!(track.layer, layer(&design, "B.Cu"));
    assert_eq!(
        track.path,
        [Point::new(mm(10.0), mm(2.0)), Point::new(mm(12.0), mm(2.0))]
    );
}

#[test]
fn syntax_errors_name_the_line() {
    let error = specctra::parse_dsn("(pcb x\n  (structure\n    (layer F.Cu)\n").unwrap_err();
    assert!(
        matches!(error, BoardError::Syntax { line: 2, .. }),
        "{error}"
    );
}

#[test]
fn pins_of_unknown_components_are_reported() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/two-resistors.dsn"
    ))
    .expect("fixture exists")
    .replace("(pins R2-1 R1-1)", "(pins R2-1 R9-1)");
    let error = specctra::parse_dsn(&text).unwrap_err();
    assert!(error.to_string().contains("R9-1"), "{error}");
}

#[test]
fn boards_from_kicad_keep_kicads_default_distance_from_the_edge() {
    // DSN cannot say how far copper must stay from the board edge; KiCad's DRC
    // applies its board setup, 0.5 mm unless the designer changed it.
    assert_eq!(two_resistors().rules.edge_clearance, mm(0.5));
}

#[test]
fn boards_from_other_tools_keep_the_board_wide_clearance_from_the_edge() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/two-resistors.dsn"
    ))
    .expect("fixture exists")
    .replace("KiCad's Pcbnew", "Some Other CAD");
    let design = specctra::parse_dsn(&text).expect("valid");
    assert_eq!(design.rules.edge_clearance, 200_100);
}
