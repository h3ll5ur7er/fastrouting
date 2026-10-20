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
        drc_report: None,
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
        drc_report: None,
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
        drc_report: None,
    })
    .expect("readable");
    assert_eq!(report.violations_by_kind(), [("clearance", 1)]);
}

#[test]
fn the_check_writes_a_drc_report_in_kicads_format_when_asked() {
    let folder = tempfile::tempdir().expect("temp dir");
    let drc = folder.path().join("report.json");
    check(&CheckJob {
        board: fixture("existing-violation.dsn"),
        session: None,
        drc_report: Some(drc.clone()),
    })
    .expect("readable");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(drc).expect("written")).expect("JSON");
    assert_eq!(report["source"], "existing-violation.dsn");
    assert_eq!(report["violations"][0]["type"], "clearance");
    assert_eq!(report["violations"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        report["unconnected_items"].as_array().map(Vec::len),
        Some(2),
        "one pad of each net apart"
    );
}
