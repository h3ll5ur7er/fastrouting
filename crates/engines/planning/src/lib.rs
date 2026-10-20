//! Planning Engine: how the work is planned (volatility V5 of
//! docs/Architecture/Volatilities.md). For now: a minimum spanning tree per net,
//! shortest connections first.

use traxide_contracts::geometry::Point;
use traxide_contracts::{Connection, Design, NetId, PadId, Plan};

pub fn plan(design: &Design) -> Plan {
    let mut connections: Vec<Connection> = design
        .nets
        .iter()
        .enumerate()
        .flat_map(|(index, net)| spanning_tree(design, NetId::from_index(index), &net.pads))
        .collect();
    // Short connections are the most constrained in where they can go, and the
    // cheapest to route, so they claim their space first.
    connections.sort_by_key(|c| {
        (
            manhattan(design.pad(c.from).position, design.pad(c.to).position),
            c.net,
            c.from,
            c.to,
        )
    });
    Plan { connections }
}

/// Prim's algorithm on Manhattan distances, which approximate rectilinear wiring
/// better than Euclidean ones. Ties go to the pad listed first.
fn spanning_tree(design: &Design, net: NetId, pads: &[PadId]) -> Vec<Connection> {
    let position = |i: usize| design.pad(pads[i]).position;
    let mut in_tree = vec![false; pads.len()];
    let mut nearest: Vec<(i64, usize)> = vec![(i64::MAX, 0); pads.len()];
    let mut connections = Vec::new();
    let mut current = 0;
    for _ in 1..pads.len() {
        in_tree[current] = true;
        for other in (0..pads.len()).filter(|&o| !in_tree[o]) {
            let distance = manhattan(position(current), position(other));
            if distance < nearest[other].0 {
                nearest[other] = (distance, current);
            }
        }
        let Some(next) = (0..pads.len())
            .filter(|&o| !in_tree[o])
            .min_by_key(|&o| (nearest[o].0, o))
        else {
            break;
        };
        connections.push(Connection {
            net,
            from: pads[nearest[next].1],
            to: pads[next],
        });
        current = next;
    }
    connections
}

fn manhattan(a: Point, b: Point) -> i64 {
    i64::from(a.x.abs_diff(b.x)) + i64::from(a.y.abs_diff(b.y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::testing::{DesignBuilder, at};

    fn pads_of(design: &Design, plan: &Plan) -> Vec<(String, String)> {
        let name = |p: PadId| design.pad(p).name.clone();
        plan.connections
            .iter()
            .map(|c| (name(c.from), name(c.to)))
            .collect()
    }

    fn pairs(names: &[(&str, &str)]) -> Vec<(String, String)> {
        names
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    }

    #[test]
    fn a_net_of_two_pads_is_one_connection() {
        let design = DesignBuilder::new(30.0, 10.0)
            .smd("A1", Some("A"), at(1.0, 1.0), 0.5, 0)
            .smd("A2", Some("A"), at(9.0, 1.0), 0.5, 0)
            .build();
        assert_eq!(pads_of(&design, &plan(&design)), pairs(&[("A1", "A2")]));
    }

    #[test]
    fn pads_in_a_row_are_chained_rather_than_joined_to_the_first() {
        let design = DesignBuilder::new(30.0, 10.0)
            .smd("A1", Some("A"), at(1.0, 1.0), 0.5, 0)
            .smd("A3", Some("A"), at(21.0, 1.0), 0.5, 0)
            .smd("A2", Some("A"), at(10.0, 1.0), 0.5, 0)
            .build();
        assert_eq!(
            pads_of(&design, &plan(&design)),
            pairs(&[("A1", "A2"), ("A2", "A3")])
        );
    }

    #[test]
    fn single_pads_need_no_connection() {
        let design = DesignBuilder::new(30.0, 10.0)
            .smd("A1", Some("A"), at(1.0, 1.0), 0.5, 0)
            .build();
        assert!(plan(&design).connections.is_empty());
    }

    #[test]
    fn the_shortest_connections_come_first() {
        let design = DesignBuilder::new(30.0, 10.0)
            .smd("L1", Some("Long"), at(1.0, 1.0), 0.5, 0)
            .smd("L2", Some("Long"), at(25.0, 1.0), 0.5, 0)
            .smd("S1", Some("Short"), at(1.0, 5.0), 0.5, 0)
            .smd("S2", Some("Short"), at(3.0, 5.0), 0.5, 0)
            .build();
        assert_eq!(
            pads_of(&design, &plan(&design)),
            pairs(&[("S1", "S2"), ("L1", "L2")])
        );
    }

    #[test]
    fn equal_distances_are_decided_by_order_not_by_chance() {
        let design = DesignBuilder::new(30.0, 30.0)
            .smd("C", Some("A"), at(10.0, 10.0), 0.5, 0)
            .smd("E", Some("A"), at(15.0, 10.0), 0.5, 0)
            .smd("W", Some("A"), at(5.0, 10.0), 0.5, 0)
            .build();
        assert_eq!(
            pads_of(&design, &plan(&design)),
            pairs(&[("C", "E"), ("C", "W")])
        );
    }
}
