use traxide_board_access::specctra::{parse_dsn, parse_ses, write_ses};
use traxide_contracts::geometry::Point;
use traxide_contracts::{Design, LayerId, NetId, Routes, Track, Via, ViaKindId};

fn design() -> Design {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/two-resistors.dsn"
    ))
    .expect("fixture exists");
    parse_dsn(&text).expect("fixture is valid")
}

fn net(design: &Design, name: &str) -> NetId {
    NetId::from_index(
        design
            .nets
            .iter()
            .position(|n| n.name == name)
            .expect("net exists"),
    )
}

fn routes(design: &Design) -> Routes {
    Routes {
        tracks: vec![
            Track {
                net: net(design, "A"),
                layer: LayerId(0),
                width: 200_000,
                path: vec![
                    Point::new(4_000_000, 5_000_000),
                    Point::new(6_000_000, 5_000_000),
                ],
            },
            Track {
                net: net(design, "Net with spaces"),
                layer: LayerId(1),
                width: 500_000,
                path: vec![
                    Point::new(1_000_000, 1_000_000),
                    Point::new(2_000_000, 2_000_000),
                ],
            },
        ],
        vias: vec![Via {
            net: net(design, "A"),
            kind: ViaKindId(0),
            position: Point::new(6_000_000, 5_000_000),
        }],
    }
}

#[test]
fn writes_wires_and_vias_per_net_in_resolution_steps_with_y_up() {
    let design = design();
    let ses = write_ses("board.ses", &design, &routes(&design));
    assert!(ses.contains("(resolution um 10)"), "{ses}");
    assert!(
        ses.contains("(wire (path F.Cu 2000 40000 -50000 60000 -50000))"),
        "{ses}"
    );
    assert!(
        ses.contains("(via \"Via[0-1]_600:300_um\" 60000 -50000)"),
        "{ses}"
    );
}

#[test]
fn quotes_names_that_contain_spaces() {
    let design = design();
    let ses = write_ses("board.ses", &design, &routes(&design));
    assert!(ses.contains("(net \"Net with spaces\""), "{ses}");
}

#[test]
fn describes_the_via_padstacks_it_uses() {
    let design = design();
    let ses = write_ses("board.ses", &design, &routes(&design));
    assert!(ses.contains("(padstack \"Via[0-1]_600:300_um\""), "{ses}");
    assert!(ses.contains("(shape (circle B.Cu 6000 0 0))"), "{ses}");
}

#[test]
fn reads_back_what_it_writes() {
    let design = design();
    let routes = routes(&design);
    let read =
        parse_ses(&write_ses("board.ses", &design, &routes), &design).expect("valid session");
    assert_eq!(read, routes);
}

#[test]
fn rounds_to_the_resolution_of_the_design() {
    let design = design();
    let mut routes = routes(&design);
    routes.vias[0].position = Point::new(6_000_049, 5_000_051);
    let read =
        parse_ses(&write_ses("board.ses", &design, &routes), &design).expect("valid session");
    assert_eq!(read.vias[0].position, Point::new(6_000_000, 5_000_100));
}

#[test]
fn rejects_nets_the_design_does_not_have() {
    let design = design();
    let ses = write_ses("board.ses", &design, &routes(&design)).replace("(net A", "(net Z");
    let error = parse_ses(&ses, &design).unwrap_err();
    assert!(error.to_string().contains("unknown net Z"), "{error}");
}

/// A session as Freerouting 2.5.0 writes one: it defines the via padstacks it
/// places in `library_out`, under names of its own (`Round1$13.779528` becomes
/// `Round1$13`), so only their copper says which via of the design they are.
fn session_defining_via(name: &str, resolution: &str, diameter: &str) -> String {
    format!(
        "(session board
  (routes
    (resolution {resolution})
    (parser (host_cad \"KiCad's Pcbnew\") (host_version 10.0.7))
    (library_out
      (padstack \"{name}\"
        (shape
          (circle F.Cu {diameter} 0 0)
        )
        (shape
          (circle B.Cu {diameter} 0 0)
        )
        (attach off)
      )
    )
    (network_out
      (net A
        (via \"{name}\" 60000 -50000
        )
      )
    )
  )
)"
    )
}

#[test]
fn a_via_padstack_the_session_defines_is_the_design_via_with_the_same_copper() {
    let design = design();
    let ses = session_defining_via("Via[0-1]_600:300", "um 10", "6000");
    let read = parse_ses(&ses, &design).expect("valid session");
    assert_eq!(
        read.vias,
        [Via {
            net: net(&design, "A"),
            kind: ViaKindId(0),
            position: Point::new(6_000_000, 5_000_000),
        }]
    );
}

#[test]
fn session_padstack_copper_matches_within_the_session_resolution() {
    let design = design();
    // 600 µm are 23622.05 steps of a thousandth of a mil: the session can only say 23622.
    let ses = session_defining_via("Round1$23", "mil 1000", "23622");
    let read = parse_ses(&ses, &design).expect("valid session");
    assert_eq!(read.vias[0].kind, ViaKindId(0));
}

#[test]
fn rejects_a_session_padstack_whose_copper_no_design_via_has() {
    let design = design();
    let ses = session_defining_via("Via[0-1]_800:400_um", "um 10", "8000");
    let error = parse_ses(&ses, &design).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("via padstack Via[0-1]_800:400_um"),
        "{error}"
    );
}

#[test]
fn a_session_padstack_named_like_a_design_via_must_also_have_its_copper() {
    let design = design();
    let ses = session_defining_via("Via[0-1]_600:300_um", "um 10", "8000");
    assert!(parse_ses(&ses, &design).is_err());
}

#[test]
fn a_via_padstack_the_session_does_not_define_is_the_design_via_of_that_name() {
    let design = design();
    let ses = "(session board (routes (resolution um 10)
        (network_out (net A (via \"Via[0-1]_600:300_um\" 60000 -50000)))))";
    let read = parse_ses(ses, &design).expect("valid session");
    assert_eq!(read.vias[0].kind, ViaKindId(0));
}
