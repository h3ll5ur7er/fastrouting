/// How to trade routing speed against quality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    /// Spacing of the routing grid.
    pub grid_pitch: i32,
    /// What a via costs, in nanometres of track it is worth avoiding.
    pub via_cost: i64,
    /// What a 45° change of direction costs, in nanometres of track.
    pub bend_cost: i64,
    /// How far a connection may stray beyond the box around its pads.
    pub detour_margin: i32,
    /// What crossing a routed connection costs, in nanometres of track, when a
    /// blocked connection looks for routes to rip up. It grows with each time
    /// that connection has been ripped up before.
    pub rip_up_cost: i64,
    /// How often one connection may be ripped up before it stays unrouted, so
    /// that two connections cannot take each other's place forever.
    pub max_rip_ups: u32,
}
