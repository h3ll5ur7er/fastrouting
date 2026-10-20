use crate::{LayerId, NetId, Track, Via};
use traxide_geometry::Segment;

/// The routing state: what copper is where, and whether more fits.
///
/// Engines use it through this trait; the manager chooses the implementation.
pub trait LayoutAccess {
    /// Whether a track segment of `net` fits on `layer` without breaking a rule.
    fn is_track_clear(&self, net: NetId, layer: LayerId, segment: Segment, width: i32) -> bool;

    /// Whether `via` fits on every layer it spans without breaking a rule.
    fn is_via_clear(&self, via: &Via) -> bool;

    fn add_track(&mut self, track: &Track);

    fn add_via(&mut self, via: &Via);
}
