//! The free ends of the fixed wiring a pad came with: a fan-out lane from a pin
//! of a fine-pitch chip runs out to where routing is meant to go on from, and a
//! route that joins the pin anywhere else leaves the lane a dangling stub.

use traxide_contracts::geometry::Point;
use traxide_contracts::{Design, LayerId, NetId, Pad, Track, Via};

/// Where the board's wiring from a pad ends without joining anything: a track's
/// end on its layer, or a via that the wiring ends in, on each of its layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FreeEnd {
    pub(crate) at: Point,
    pub(crate) layers: Vec<LayerId>,
}

/// The free ends of the existing wiring that `pad` starts, followed along
/// tracks that meet end to end and through vias, and not through other pads of
/// the net, where the wiring is a connection already made.
pub(crate) fn free_ends(design: &Design, pad: &Pad) -> Vec<FreeEnd> {
    let Some(net) = pad.net else {
        return Vec::new();
    };
    let wiring = Wiring::of(design, net);
    let mut seen_tracks = vec![false; wiring.tracks.len()];
    let mut seen_vias = vec![false; wiring.vias.len()];
    let mut to_follow: Vec<(usize, Point)> = Vec::new();
    for (i, track) in wiring.tracks.iter().enumerate() {
        for end in ends(track) {
            if covers(pad, track.layer, end) && !seen_tracks[i] {
                seen_tracks[i] = true;
                to_follow.push((i, end));
            }
        }
    }
    let mut found = Vec::new();
    while let Some((i, entered_at)) = to_follow.pop() {
        let track = wiring.tracks[i];
        let Some(far) = ends(track).into_iter().find(|&end| end != entered_at) else {
            continue;
        };
        if wiring.on_a_pad(track.layer, far) {
            continue;
        }
        let mut joined = false;
        for j in wiring.tracks_ending_at(track.layer, far, i) {
            joined = true;
            if !seen_tracks[j] {
                seen_tracks[j] = true;
                to_follow.push((j, far));
            }
        }
        for v in wiring.vias_covering(track.layer, far) {
            joined = true;
            if seen_vias[v] {
                continue;
            }
            seen_vias[v] = true;
            let via = wiring.vias[v];
            let layers = wiring.layers_of(via);
            if layers.iter().any(|&l| wiring.on_a_pad(l, via.position)) {
                continue;
            }
            let onward: Vec<(usize, Point)> = layers
                .iter()
                .flat_map(|&layer| wiring.tracks_ending_in_via(v, layer, i))
                .collect();
            if onward.is_empty() {
                found.push(FreeEnd {
                    at: via.position,
                    layers,
                });
            }
            for (j, end) in onward {
                if !seen_tracks[j] {
                    seen_tracks[j] = true;
                    to_follow.push((j, end));
                }
            }
        }
        if !joined {
            found.push(FreeEnd {
                at: far,
                layers: vec![track.layer],
            });
        }
    }
    found
}

struct Wiring<'d> {
    design: &'d Design,
    net: NetId,
    tracks: Vec<&'d Track>,
    vias: Vec<&'d Via>,
}

impl<'d> Wiring<'d> {
    fn of(design: &'d Design, net: NetId) -> Self {
        Self {
            design,
            net,
            tracks: design
                .existing
                .tracks
                .iter()
                .filter(|t| t.net == net && t.path.len() >= 2)
                .collect(),
            vias: design
                .existing
                .vias
                .iter()
                .filter(|v| v.net == net)
                .collect(),
        }
    }

    fn on_a_pad(&self, layer: LayerId, point: Point) -> bool {
        self.design
            .net(self.net)
            .pads
            .iter()
            .any(|&id| covers(self.design.pad(id), layer, point))
    }

    fn tracks_ending_at(
        &self,
        layer: LayerId,
        point: Point,
        except: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        self.tracks
            .iter()
            .enumerate()
            .filter(move |&(j, t)| j != except && t.layer == layer && ends(t).contains(&point))
            .map(|(j, _)| j)
    }

    fn vias_covering(&self, layer: LayerId, point: Point) -> impl Iterator<Item = usize> + '_ {
        (0..self.vias.len()).filter(move |&v| self.via_covers(v, layer, point))
    }

    fn tracks_ending_in_via(
        &self,
        via: usize,
        layer: LayerId,
        except: usize,
    ) -> impl Iterator<Item = (usize, Point)> + '_ {
        self.tracks
            .iter()
            .enumerate()
            .filter(move |&(j, t)| j != except && t.layer == layer)
            .flat_map(move |(j, t)| ends(t).into_iter().map(move |end| (j, end)))
            .filter(move |&(_, end)| self.via_covers(via, layer, end))
    }

    fn via_covers(&self, via: usize, layer: LayerId, point: Point) -> bool {
        let via = self.vias[via];
        self.design
            .via_kind(via.kind)
            .copper
            .iter()
            .filter(|c| c.layer == layer)
            .any(|c| {
                let shape = c.shape.map(|p| p + via.position);
                shape.nearest_point(point) == point
            })
    }

    fn layers_of(&self, via: &Via) -> Vec<LayerId> {
        self.design
            .via_kind(via.kind)
            .copper
            .iter()
            .map(|c| c.layer)
            .collect()
    }
}

fn ends(track: &Track) -> [Point; 2] {
    [track.path[0], track.path[track.path.len() - 1]]
}

fn covers(pad: &Pad, layer: LayerId, point: Point) -> bool {
    pad.copper
        .iter()
        .any(|c| c.layer == layer && c.shape.nearest_point(point) == point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use traxide_contracts::ViaKindId;
    use traxide_contracts::testing::{DesignBuilder, at, mm};

    fn track(design: &Design, layer: u32, path: &[Point]) -> Track {
        Track {
            net: design.net_named("A"),
            layer: LayerId(layer),
            width: mm(0.2),
            path: path.to_vec(),
        }
    }

    fn board() -> Design {
        DesignBuilder::new(20.0, 10.0)
            .smd("A1", Some("A"), at(5.0, 5.0), 1.0, 0)
            .smd("A2", Some("A"), at(15.0, 5.0), 1.0, 0)
            .build()
    }

    #[test]
    fn a_lane_of_two_tracks_ends_where_the_second_does() {
        let mut design = board();
        design.existing.tracks = vec![
            track(&design, 0, &[at(5.0, 5.0), at(6.0, 6.0)]),
            track(&design, 0, &[at(6.0, 6.0), at(8.0, 6.0)]),
        ];
        let pad = design.pad(design.pad_named("A1"));
        assert_eq!(
            free_ends(&design, pad),
            [FreeEnd {
                at: at(8.0, 6.0),
                layers: vec![LayerId(0)]
            }]
        );
    }

    #[test]
    fn a_lane_that_ends_in_a_via_ends_on_each_of_its_layers() {
        let mut design = board();
        design.existing.tracks = vec![track(&design, 0, &[at(5.0, 5.0), at(7.0, 5.0)])];
        design.existing.vias = vec![Via {
            net: design.net_named("A"),
            kind: ViaKindId(0),
            position: at(7.0, 5.0),
        }];
        let pad = design.pad(design.pad_named("A1"));
        assert_eq!(
            free_ends(&design, pad),
            [FreeEnd {
                at: at(7.0, 5.0),
                layers: vec![LayerId(0), LayerId(1)]
            }]
        );
    }

    #[test]
    fn wiring_that_reaches_another_pad_of_the_net_has_no_free_end() {
        let mut design = board();
        design.existing.tracks = vec![track(&design, 0, &[at(5.0, 5.0), at(15.0, 5.0)])];
        let pad = design.pad(design.pad_named("A1"));
        assert_eq!(free_ends(&design, pad), []);
    }
}
