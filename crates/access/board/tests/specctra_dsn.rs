use traxide_board_access::{BoardError, load_design, specctra};
use traxide_contracts::geometry::{Point, Rect, Shape};
use traxide_contracts::{Design, KeepoutScope, LayerId, Pad};

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
    assert_eq!(design.outline.polygons()[0].points().len(), 4);
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

#[test]
fn keepouts_of_a_footprint_move_with_it() {
    // KiCad exports a footprint's non-plated holes, such as a switch's centre
    // post, as keepouts of its image.
    let design = two_resistors();
    let hole = design
        .keepouts
        .iter()
        .find(|k| k.shape == Shape::circle(Point::new(mm(25.0), mm(12.0)), mm(2.0)))
        .expect("the keepout of R2's image, 3 mm above its origin");
    assert_eq!(hole.layer, Some(layer(&design, "F.Cu")));
    assert_eq!(hole.scope, KeepoutScope::Everything);
}

#[test]
fn keepouts_of_a_footprint_on_the_back_mirror_with_it() {
    let design = two_resistors();
    // R3 sits on the back, rotated by 90°: the image's (0, 1 mm) lands 1 mm to its left.
    let r3 = design
        .keepouts
        .iter()
        .find(|k| k.shape == Shape::circle(Point::new(mm(14.0), mm(10.0)), mm(0.5)))
        .expect("the via keepout of R3's image");
    assert_eq!(r3.layer, Some(layer(&design, "B.Cu")));
    assert_eq!(r3.scope, KeepoutScope::ViasOnly);
}

/// `fixture` from the workspace's boards, with `edit` applied to its bytes.
fn edited_board(fixture: &str, edit: impl Fn(Vec<u8>) -> Vec<u8>) -> tempfile::TempPath {
    let original = std::fs::read(format!(
        "{}/../../../fixtures/boards/{fixture}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture");
    let path = tempfile::Builder::new()
        .suffix(".dsn")
        .tempfile()
        .expect("temp file")
        .into_temp_path();
    std::fs::write(&path, edit(original)).expect("writable");
    path
}

fn replaced(bytes: Vec<u8>, from: &[u8], to: &[u8]) -> Vec<u8> {
    let at = bytes
        .windows(from.len())
        .position(|w| w == from)
        .expect("present");
    [&bytes[..at], to, &bytes[at + from.len()..]].concat()
}

#[test]
fn reads_designs_written_in_latin_1() {
    // "Across" renamed to "Acrèss" in Latin-1, where è is the single byte E8.
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(bytes, b"(net Across", b"(net Acr\xe8ss")
    });
    let design = load_design(&board).expect("readable");
    assert!(design.nets.iter().any(|n| n.name == "Acrèss"));
}

#[test]
fn a_file_that_is_no_specctra_design_says_so() {
    let board = edited_board("two-nets.dsn", |_| {
        b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1 binary".to_vec()
    });
    let error = load_design(&board).unwrap_err();
    assert!(
        error.to_string().contains("not a Specctra design"),
        "{error}"
    );
}

#[test]
fn rules_may_be_spread_over_several_lists_and_say_clear_for_clearance() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(rule (width 200) (clearance 200))",
            b"(rule (width 200)) (rule (clear 300)) (rule (clear 100 (type smd_smd)))",
        )
    });
    let design = load_design(&board).expect("readable");
    let class = &design.rules.classes[design.rules.default_class.index()];
    assert_eq!((class.track_width, class.clearance), (mm(0.2), mm(0.3)));
}

#[test]
fn the_signal_boundary_is_where_routing_may_go() {
    // The board is 30 × 20 mm; its signal boundary keeps 1 mm inside.
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(via ",
            b"(boundary (rect signal 1000 -19000 29000 -1000))\n    (via ",
        )
    });
    let design = load_design(&board).expect("readable");
    assert_eq!(
        design.outline.bounds(),
        Rect {
            min: Point::new(mm(1.0), mm(1.0)),
            max: Point::new(mm(29.0), mm(19.0)),
        }
    );
}

#[test]
fn a_polyline_path_lists_lines_whose_crossings_are_its_corners() {
    // As Freerouting writes wires: a horizontal line through the start, a
    // vertical one along the wire, and a horizontal one through its end.
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(wiring)",
            b"(wiring (wire (polyline_path F.Cu 200  6000 -10000 6000.1 -10000  \
              6000 -10000 6000 -10000.1  0 -14000 0.1 -14000) (net Across)))",
        )
    });
    let design = load_design(&board).expect("readable");
    assert_eq!(
        design.existing.tracks[0].path,
        [Point::new(mm(6.0), mm(10.0)), Point::new(mm(6.0), mm(14.0))]
    );
}

#[test]
fn pins_of_a_net_may_quote_their_pin_names() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(bytes, b"(pins R1-2 R2-1)", b"(pins R1-\"2\" R2-1)")
    });
    let design = load_design(&board).expect("readable");
    let across = design.nets.iter().position(|n| n.name == "Across");
    assert_eq!(pad(&design, "R1-2").net.map(|n| n.index()), across);
}

#[test]
fn a_pad_polygon_of_two_points_is_a_line_as_wide_as_its_aperture() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(shape (rect F.Cu -500 -600 500 600))",
            b"(shape (polygon F.Cu 100 0 -500 0 500))",
        )
    });
    let design = load_design(&board).expect("readable");
    let copper = &pad(&design, "R1-1").copper[0];
    assert!(
        matches!(copper.shape, Shape::Capsule { radius: 50_000, .. }),
        "{copper:?}"
    );
}

#[test]
fn reads_circ_as_circle() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(shape (rect F.Cu -500 -600 500 600))",
            b"(shape (circ F.Cu 1000))",
        )
    });
    let design = load_design(&board).expect("readable");
    let copper = &pad(&design, "R1-1").copper[0];
    assert!(
        matches!(
            copper.shape,
            Shape::Capsule {
                radius: 500_000,
                ..
            }
        ),
        "{copper:?}"
    );
}

#[test]
fn a_class_without_a_name_or_nets_changes_nothing() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(net Down (pins R3-1 R4-2))",
            b"(net Down (pins R3-1 R4-2))\n    (class (circuit (use_via \"Via[0-1]_600:300_um\")) (rule (width 900)))",
        )
    });
    let design = load_design(&board).expect("readable");
    assert!(
        design
            .nets
            .iter()
            .all(|n| n.class == design.rules.default_class)
    );
}

#[test]
fn a_polygon_whose_points_coincide_is_a_dot() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(via ",
            b"(keepout \"\" (polygon signal 0  1000 -1000  1000 -1000  1000 -1000))\n    (via ",
        )
    });
    let design = load_design(&board).expect("readable");
    assert!(matches!(
        design.keepouts[0].shape,
        Shape::Capsule { radius: 0, .. }
    ));
}

#[test]
fn keepouts_of_holes_kicad_7_0_2_wrote_half_a_metre_too_wide_get_their_size_back() {
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"(pin Rect[T]Pad_1000x1200_um 1 -1000 0)",
            b"(keepout \"\" (circle F.Cu 501100))\n      (pin Rect[T]Pad_1000x1200_um 1 -1000 0)",
        )
    });
    let design = load_design(&board).expect("readable");
    assert!(
        design.keepouts.iter().all(|k| matches!(
            k.shape,
            Shape::Capsule {
                radius: 550_000,
                ..
            }
        )),
        "{:?}",
        design.keepouts
    );
}

#[test]
fn a_boundary_of_two_paths_is_a_panel_of_two_boards() {
    // The 30 × 20 mm board, and a second one 10 mm below it.
    let board = edited_board("two-nets.dsn", |bytes| {
        replaced(
            bytes,
            b"30000 -20000  0 -20000  0 0  30000 0  30000 -20000)",
            b"30000 -20000  0 -20000  0 0  30000 0  30000 -20000)\n      (path pcb 0  30000 -50000  0 -50000  0 -30000  30000 -30000)",
        )
    });
    let design = load_design(&board).expect("readable");
    assert!(design.outline.contains(Point::new(mm(15.0), mm(10.0))));
    assert!(design.outline.contains(Point::new(mm(15.0), mm(40.0))));
    assert!(!design.outline.contains(Point::new(mm(15.0), mm(25.0))));
}

fn two_resistors_text() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/two-resistors.dsn"
    ))
    .expect("fixture exists")
}

/// The fixture with its bottom layer of type power, and `structure` added.
fn with_power_bottom(text: &str, structure: &str) -> Design {
    let at = text.find("(layer B.Cu").expect("bottom layer");
    let typed = at + text[at..].find("(type signal)").expect("typed");
    let text = format!(
        "{}(type power){}",
        &text[..typed],
        &text[typed + "(type signal)".len()..]
    )
    .replacen("(boundary", &format!("{structure}\n    (boundary"), 1);
    specctra::parse_dsn(&text).expect("valid")
}

#[test]
fn a_power_layer_from_kicad_that_holds_no_plane_takes_tracks() {
    // KiCad writes `power` for a copper layer of that type, on which it routes
    // and checks tracks like on any other.
    let design = with_power_bottom(&two_resistors_text(), "");
    assert!(design.layers[layer(&design, "B.Cu").index()].routable);
}

#[test]
fn a_power_layer_that_holds_a_plane_takes_no_tracks() {
    let plane = "(plane GND (polygon B.Cu 0  30000 -20000  0 -20000  0 0  30000 0))";
    let design = with_power_bottom(&two_resistors_text(), plane);
    assert!(!design.layers[layer(&design, "B.Cu").index()].routable);
}

#[test]
fn a_power_layer_from_another_tool_is_a_plane() {
    let text = two_resistors_text().replace("KiCad's Pcbnew", "Some Other CAD");
    let design = with_power_bottom(&text, "");
    assert!(!design.layers[layer(&design, "B.Cu").index()].routable);
}

#[test]
fn a_pin_listed_in_two_nets_stays_in_the_last_and_is_named() {
    let text = two_resistors_text().replace("(pins R2-1 R1-1)", "(pins R2-1 R1-1 R3-1)");
    let design = specctra::parse_dsn(&text).expect("valid");
    let r3 = pad(&design, "R3-1");
    assert_eq!(
        design.net(r3.net.expect("in a net")).name,
        "Net with spaces"
    );
    let a = design.nets.iter().find(|n| n.name == "A").expect("net A");
    assert!(a.pads.iter().all(|&p| design.pad(p).name != "R3-1"));
    assert_eq!(
        design.warnings,
        ["pin R3-1 is listed in the nets A and Net with spaces; it stays in Net with spaces"]
    );
    assert!(two_resistors().warnings.is_empty());
}

#[test]
fn a_pin_listed_twice_in_one_net_stays_in_it_without_a_word() {
    let text = two_resistors_text().replace("(pins R2-1 R1-1)", "(pins R2-1 R1-1 R1-1)");
    let design = specctra::parse_dsn(&text).expect("valid");
    assert!(design.warnings.is_empty(), "{:?}", design.warnings);
    assert_eq!(
        design.net(pad(&design, "R1-1").net.expect("in a net")).name,
        "A"
    );
}

#[test]
fn parts_placed_wholly_outside_the_outline_are_named() {
    let folder = tempfile::tempdir().expect("temp dir");
    let path = folder.path().join("outside.dsn");
    let text = two_resistors_text()
        .replace(
            "(place R2 25000.000000 -15000.000000",
            "(place R2 55000.000000 -15000.000000",
        )
        .replace(
            "(place R1 5000.000000 -5000.000000",
            "(place R1 45000.000000 -5000.000000",
        );
    std::fs::write(&path, text).expect("written");
    let design = load_design(&path).expect("valid");
    assert_eq!(
        design.warnings,
        ["the parts R1, R2 lie outside the board's outline"]
    );
}

#[test]
fn reads_planes_with_their_net_layer_and_outline() {
    let plane = "(plane A (polygon B.Cu 0  1000 -1000  29000 -1000  29000 -19000  1000 -19000))";
    let design = with_power_bottom(&two_resistors_text(), plane);
    let [plane] = &design.planes[..] else {
        panic!("one plane: {:?}", design.planes);
    };
    assert_eq!(design.net(plane.net).name, "A");
    assert_eq!(plane.layer, layer(&design, "B.Cu"));
    assert_eq!(
        plane.outline.bounds(),
        Rect {
            min: Point::new(mm(1.0), mm(1.0)),
            max: Point::new(mm(29.0), mm(19.0)),
        }
    );
}

#[test]
fn a_plane_of_a_net_the_network_does_not_list_is_left_out() {
    let plane = "(plane GND (polygon B.Cu 0  30000 -20000  0 -20000  0 0  30000 0))";
    let design = with_power_bottom(&two_resistors_text(), plane);
    assert!(design.planes.is_empty(), "{:?}", design.planes);
}

#[test]
fn a_window_in_a_plane_is_a_hole_of_it() {
    // KiCad writes a window for every hole of a zone, as TokenBoard's VLED
    // plane has one where its +3V3 island sits.
    let plane = "(plane A (polygon B.Cu 0  1000 -1000  29000 -1000  29000 -19000  1000 -19000)
                   (window (polygon B.Cu 0  10000 -5000  20000 -5000  20000 -15000  10000 -15000)))";
    let design = with_power_bottom(&two_resistors_text(), plane);
    let [plane] = &design.planes[..] else {
        panic!("one plane: {:?}", design.planes);
    };
    let [hole] = &plane.holes[..] else {
        panic!("one hole: {:?}", plane.holes);
    };
    assert_eq!(
        hole.bounds(),
        Rect {
            min: Point::new(mm(10.0), mm(5.0)),
            max: Point::new(mm(20.0), mm(15.0)),
        }
    );
}

#[test]
fn a_window_in_the_boundary_is_a_hole_in_the_board() {
    let text = two_resistors_text().replace(
        "(path pcb 0  30000 -20000  0 -20000  0 0  30000 0  30000 -20000)",
        "(path pcb 0  30000 -20000  0 -20000  0 0  30000 0  30000 -20000)
      (window (rect pcb 10000 -5000 12000 -7000))",
    );
    let design = specctra::parse_dsn(&text).expect("valid");
    assert_eq!(design.outline.holes().len(), 1);
    assert!(!design.outline.contains(Point::new(mm(11.0), mm(6.0))));
    assert!(design.outline.contains(Point::new(mm(5.0), mm(6.0))));
}
