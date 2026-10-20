//! Detailed Routing Engine: the algorithm that turns connections into exact,
//! rule-correct copper (volatility V7 of docs/Architecture/Volatilities.md).
//!
//! For now an A* search on an octilinear grid across all routable layers, one
//! connection at a time against everything routed before it.

mod grid;
mod path;
mod scratch;
mod search;

use traxide_contracts::{Connection, ConstraintSet, Design, LayoutAccess, Profile, Routes};

pub struct Router<'a> {
    design: &'a Design,
    rules: &'a ConstraintSet,
    profile: &'a Profile,
    scratch: search::Scratch,
}

impl std::fmt::Debug for Router<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("profile", &self.profile.name)
            .finish_non_exhaustive()
    }
}

impl<'a> Router<'a> {
    pub fn new(design: &'a Design, rules: &'a ConstraintSet, profile: &'a Profile) -> Self {
        Self {
            design,
            rules,
            profile,
            scratch: search::Scratch::default(),
        }
    }

    /// Routes `connection` and adds its copper to `layout`; `None` when no path fits.
    pub fn route(
        &mut self,
        connection: Connection,
        layout: &mut impl LayoutAccess,
    ) -> Option<Routes> {
        let search = search::Search::new(self.design, self.rules, self.profile, connection, layout);
        let steps = search.run(&mut self.scratch)?;
        let routes = path::to_routes(
            &steps,
            connection.net,
            self.rules.track_width(connection.net),
        );
        for track in &routes.tracks {
            layout.add_track(track);
        }
        for via in &routes.vias {
            layout.add_via(via);
        }
        Some(routes)
    }
}
