use std::cell::RefCell;
use std::path::PathBuf;
use traxide_contracts::{Event, Stage};
use traxide_events::{CancelToken, Silent};
use traxide_routing::{RouteJob, route};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../../fixtures/boards/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn job(input: &str, output: &std::path::Path) -> RouteJob {
    RouteJob {
        input: fixture(input),
        output: output.into(),
        profile: "default".into(),
        profile_file: None,
        threads: None,
        unrouted_net_classes: Vec::new(),
    }
}

#[test]
fn routes_every_connection_and_stores_a_clean_session() {
    let folder = tempfile::tempdir().expect("temp dir");
    let output = folder.path().join("board.ses");
    let report =
        route(&job("two-nets.dsn", &output), &Silent, &CancelToken::new()).expect("routable");
    assert_eq!((report.connections, report.unrouted.len()), (2, 0));
    assert!(report.verdict.is_clean(), "{:?}", report.verdict);
    assert!(
        std::fs::read_to_string(output)
            .expect("stored")
            .contains("(network_out")
    );
}

#[test]
fn tells_its_client_what_it_is_doing() {
    let folder = tempfile::tempdir().expect("temp dir");
    let events = RefCell::new(Vec::new());
    let collect = |e: Event| events.borrow_mut().push(e);
    route(
        &job("two-nets.dsn", &folder.path().join("board.ses")),
        &collect,
        &CancelToken::new(),
    )
    .expect("routable");
    let events = events.into_inner();
    assert_eq!(events.first(), Some(&Event::StageStarted(Stage::Loading)));
    assert!(events.contains(&Event::ConnectionRouted { done: 2, total: 2 }));
    assert_eq!(events.last(), Some(&Event::StageStarted(Stage::Storing)));
}

#[test]
fn a_cancelled_job_stores_what_it_has_and_lists_the_rest_as_unrouted() {
    let folder = tempfile::tempdir().expect("temp dir");
    let cancel = CancelToken::new();
    cancel.cancel();
    let report = route(
        &job("two-nets.dsn", &folder.path().join("board.ses")),
        &Silent,
        &cancel,
    )
    .expect("stores");
    assert!(report.cancelled);
    assert_eq!(report.unrouted.len(), 2);
}

#[test]
fn a_missing_board_is_an_error_that_names_it() {
    let folder = tempfile::tempdir().expect("temp dir");
    let error = route(
        &job("missing.dsn", &folder.path().join("board.ses")),
        &Silent,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("missing.dsn"), "{error}");
}

#[test]
fn nets_of_the_net_classes_left_unrouted_get_no_copper() {
    let folder = tempfile::tempdir().expect("temp dir");
    let output = folder.path().join("board.ses");
    let job = RouteJob {
        unrouted_net_classes: vec!["Power".into()],
        ..job("two-classes.dsn", &output)
    };
    let report = route(&job, &Silent, &CancelToken::new()).expect("routable");
    assert_eq!((report.connections, report.unrouted.len()), (1, 0));
    assert_eq!(report.tracks, 1, "only Across, straight across");
}

#[test]
fn an_unknown_net_class_to_leave_unrouted_is_an_error_naming_the_known_ones() {
    let folder = tempfile::tempdir().expect("temp dir");
    let job = RouteJob {
        unrouted_net_classes: vec!["Pwr".into()],
        ..job("two-classes.dsn", &folder.path().join("board.ses"))
    };
    let error = route(&job, &Silent, &CancelToken::new()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "no net class named Pwr; there are default, kicad_default, Power"
    );
}

#[test]
fn rules_the_board_broke_before_routing_are_not_counted_as_the_routers() {
    // An existing track of Across crosses a pad of Down.
    let folder = tempfile::tempdir().expect("temp dir");
    let output = folder.path().join("board.ses");
    let report = route(
        &job("existing-violation.dsn", &output),
        &Silent,
        &CancelToken::new(),
    )
    .expect("routable");
    assert_eq!(report.violations_before_routing, 1);
    assert!(
        report
            .verdict
            .violations
            .iter()
            .all(|v| matches!(v, traxide_contracts::Violation::Unconnected { .. })),
        "{:?}",
        report.verdict
    );
}
