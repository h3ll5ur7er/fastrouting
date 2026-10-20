use std::path::PathBuf;
use traxide_inspection::{CheckJob, check};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../../fixtures/boards/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn a_board_without_routes_has_every_net_unconnected() {
    let report = check(&CheckJob {
        board: fixture("two-nets.dsn"),
        session: None,
    })
    .expect("readable");
    assert_eq!((report.nets, report.incomplete_nets), (2, 2));
    assert_eq!(report.unconnected_pads, 2);
    assert!(report.violations.is_empty());
}

#[test]
fn a_board_with_its_routes_is_complete_and_clean() {
    let session = tempfile::tempdir()
        .expect("temp dir")
        .keep()
        .join("routed.ses");
    std::fs::write(
        &session,
        "(session routed.ses (base_design two-nets.dsn)
           (routes (resolution um 10) (parser)
             (network_out
               (net Across (wire (path F.Cu 2000 60000 -100000 240000 -100000)))
               (net Down
                 (wire (path F.Cu 2000 150000 -50000 150000 -80000))
                 (via \"Via[0-1]_600:300_um\" 150000 -80000)
                 (wire (path B.Cu 2000 150000 -80000 150000 -120000))
                 (via \"Via[0-1]_600:300_um\" 150000 -120000)
                 (wire (path F.Cu 2000 150000 -120000 150000 -150000))))))",
    )
    .expect("writable");
    let report = check(&CheckJob {
        board: fixture("two-nets.dsn"),
        session: Some(session),
    })
    .expect("readable");
    assert_eq!(report.incomplete_nets, 0, "{report:?}");
    assert!(report.violations.is_empty(), "{report:?}");
    assert_eq!((report.tracks, report.vias), (4, 2));
}

#[test]
fn copper_that_breaks_a_rule_is_reported_by_kind() {
    let report = check(&CheckJob {
        board: fixture("existing-violation.dsn"),
        session: None,
    })
    .expect("readable");
    assert_eq!(report.violations_by_kind(), [("clearance", 1)]);
}
