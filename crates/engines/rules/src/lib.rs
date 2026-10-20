//! Rules Engine: what the design rules mean (volatility V4 of
//! docs/Architecture/Volatilities.md), compiled once into a [`ConstraintSet`].

use traxide_contracts::{ConstraintSet, Design};

/// What the compiled rules are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// For routing on a grid of `grid_pitch`. Stored routes are rounded to the
    /// design's resolution, which can move copper towards its neighbours by up
    /// to a step: clearances grow by one, unless the grid is made of whole steps,
    /// as on boards from KiCad. Then nothing moves, and copper may fit exactly
    /// where the rules allow, as between the pins of a fine-pitch chip.
    Routing { grid_pitch: i32 },
    /// The rules exactly as the design states them, as KiCad's DRC applies them.
    Verification,
}

pub fn compile(design: &Design, purpose: Purpose) -> ConstraintSet {
    let classes = &design.rules.classes;
    let margin = match purpose {
        Purpose::Routing { grid_pitch }
            if design.resolution > 0 && grid_pitch % design.resolution != 0 =>
        {
            design.resolution
        }
        Purpose::Routing { .. } | Purpose::Verification => 0,
    };
    let clearance_between_classes = classes
        .iter()
        .flat_map(|a| {
            classes
                .iter()
                .map(move |b| a.clearance.max(b.clearance) + margin)
        })
        .collect();
    ConstraintSet::new(
        design.nets.iter().map(|net| net.class).collect(),
        design.rules.default_class,
        classes.iter().map(|class| class.track_width).collect(),
        classes
            .iter()
            .map(|class| class.via_kinds.clone())
            .collect(),
        clearance_between_classes,
        design.rules.edge_clearance + margin,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::geometry::{Point, Polygon, Rect};
    use traxide_contracts::{DesignRules, Net, NetClass, NetClassId, NetId, Routes, ViaKindId};

    fn class(name: &str, track_width: i32, clearance: i32, via: u32) -> NetClass {
        NetClass {
            name: name.into(),
            track_width,
            clearance,
            via_kinds: vec![ViaKindId(via)],
        }
    }

    fn design(resolution: i32) -> Design {
        Design {
            layers: vec![],
            outline: Polygon::rectangle(Rect {
                min: Point::new(0, 0),
                max: Point::new(1, 1),
            })
            .into(),
            nets: vec![
                Net {
                    name: "signal".into(),
                    class: NetClassId(0),
                    pads: vec![],
                },
                Net {
                    name: "power".into(),
                    class: NetClassId(1),
                    pads: vec![],
                },
            ],
            pads: vec![],
            via_kinds: vec![],
            keepouts: vec![],
            existing: Routes::default(),
            rules: DesignRules {
                classes: vec![class("default", 200, 200, 0), class("power", 500, 300, 1)],
                default_class: NetClassId(0),
                edge_clearance: 250,
            },
            resolution,
        }
    }

    const SIGNAL: Option<NetId> = Some(NetId(0));
    const POWER: Option<NetId> = Some(NetId(1));

    #[test]
    fn nets_of_two_classes_keep_the_larger_clearance() {
        let rules = compile(&design(0), Purpose::Routing { grid_pitch: 100 });
        assert_eq!(rules.clearance(SIGNAL, POWER), 300);
        assert_eq!(rules.clearance(POWER, SIGNAL), 300);
        assert_eq!(rules.clearance(SIGNAL, SIGNAL), 200);
    }

    #[test]
    fn copper_of_no_net_follows_the_default_class() {
        assert_eq!(
            compile(&design(0), Purpose::Routing { grid_pitch: 100 }).clearance(None, SIGNAL),
            200
        );
    }

    #[test]
    fn clearances_include_the_rounding_of_stored_routes_off_the_resolution() {
        let rules = compile(&design(100), Purpose::Routing { grid_pitch: 150 });
        assert_eq!(rules.clearance(SIGNAL, POWER), 400);
        assert_eq!(rules.edge_clearance(), 350);
    }

    #[test]
    fn routes_on_a_grid_of_whole_resolution_steps_may_fit_exactly() {
        let rules = compile(&design(100), Purpose::Routing { grid_pitch: 1_000 });
        assert_eq!(rules.clearance(SIGNAL, POWER), 300);
        assert_eq!(rules.edge_clearance(), 250);
    }

    #[test]
    fn verification_applies_the_rules_as_stated() {
        let rules = compile(&design(100), Purpose::Verification);
        assert_eq!(rules.clearance(SIGNAL, POWER), 300);
        assert_eq!(rules.edge_clearance(), 250);
    }

    #[test]
    fn widths_and_vias_follow_the_class_of_the_net() {
        let rules = compile(&design(100), Purpose::Routing { grid_pitch: 100 });
        assert_eq!(rules.track_width(NetId(1)), 500);
        assert_eq!(rules.via_kinds(NetId(1)), [ViaKindId(1)]);
    }

    #[test]
    fn the_largest_clearance_bounds_every_query() {
        assert_eq!(
            compile(&design(100), Purpose::Routing { grid_pitch: 150 }).largest_clearance(),
            400
        );
    }
}
