//! Routing Manager: the workflow of routing a board (volatility V2 of
//! docs/Architecture/Volatilities.md). It decides which stages run in which
//! order; the engines and resource access do the work.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use traxide_board_access::BoardError;
use traxide_contracts::{Connection, Event, RoutingResult, Stage, Verdict};
use traxide_detailed::Router;
use traxide_events::{CancelToken, Publisher};
use traxide_layout_access::Layout;
use traxide_profile_access::ProfileError;
use traxide_rules::Purpose;

/// Route the board in `input` and store the routes in `output`.
#[derive(Debug, Clone)]
pub struct RouteJob {
    pub input: PathBuf,
    pub output: PathBuf,
    pub profile: String,
    pub profile_file: Option<PathBuf>,
}

/// What a finished route job achieved.
#[derive(Debug, Clone)]
pub struct RouteReport {
    pub connections: usize,
    /// Each as the net's name and the names of its two pads.
    pub unrouted: Vec<(String, String, String)>,
    pub tracks: usize,
    pub vias: usize,
    /// The independent check of the stored routes. Unconnected pads follow from
    /// unrouted connections; any other violation is a bug of the router.
    pub verdict: Verdict,
    pub cancelled: bool,
    pub elapsed: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error(transparent)]
    Board(#[from] BoardError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
}

/// Routes every connection of the board in `job`, storing what fits.
pub fn route(
    job: &RouteJob,
    events: &impl Publisher<Event>,
    cancel: &CancelToken,
) -> Result<RouteReport, RouteError> {
    let started = Instant::now();
    let profile = traxide_profile_access::load_profile(&job.profile, job.profile_file.as_deref())?;
    events.publish(Event::StageStarted(Stage::Loading));
    let design = traxide_board_access::load_design(&job.input)?;
    events.publish(Event::StageStarted(Stage::CompilingRules));
    let rules = traxide_rules::compile(&design, Purpose::Routing);
    events.publish(Event::StageStarted(Stage::BuildingLayout));
    let mut layout = Layout::build(&design, &rules);
    events.publish(Event::StageStarted(Stage::Planning));
    let plan = traxide_planning::plan(&design);
    let corridors = profile.global_routing.then(|| {
        events.publish(Event::StageStarted(Stage::GlobalRouting));
        traxide_global::route(&design, &plan, &rules, &profile)
    });
    events.publish(Event::StageStarted(Stage::Routing));
    let total = plan.connections.len();
    let mut router = Router::new(&design, &rules, &profile);
    let RoutingResult { routes, unrouted } =
        router.route_plan(&plan, corridors.as_ref(), &mut layout, cancel, events);
    events.publish(Event::StageStarted(Stage::Verifying));
    let mut everything = design.existing.clone();
    everything.extend(routes.clone());
    let verdict = traxide_verification::verify(
        &design,
        &traxide_rules::compile(&design, Purpose::Verification),
        &everything,
    );
    events.publish(Event::StageStarted(Stage::Storing));
    traxide_board_access::store_routes(&job.output, &design, &routes)?;
    let named = |c: &Connection| {
        (
            design.net(c.net).name.clone(),
            design.pad(c.from).name.clone(),
            design.pad(c.to).name.clone(),
        )
    };
    Ok(RouteReport {
        connections: total,
        unrouted: unrouted.iter().map(named).collect(),
        tracks: routes.tracks.len(),
        vias: routes.vias.len(),
        verdict,
        cancelled: cancel.is_cancelled(),
        elapsed: started.elapsed(),
    })
}
