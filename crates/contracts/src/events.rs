/// What a manager tells its client while a job runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    StageStarted(Stage),
    ConnectionRouted { done: usize, total: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Loading,
    CompilingRules,
    BuildingLayout,
    Planning,
    Routing,
    Storing,
}
