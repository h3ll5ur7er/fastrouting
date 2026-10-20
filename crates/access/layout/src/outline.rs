use crate::buckets::Buckets;
use traxide_contracts::geometry::{Polygon, Rect, Segment, Shape};

/// The board's outline, indexed so that a question about a piece of copper
/// looks only at the edges near it. Boards with rounded corners have outlines
/// of hundreds of edges, and testing them all made every check of every
/// search slow.
#[derive(Debug)]
pub(crate) struct Outline {
    polygon: Polygon,
    bounds: Rect,
    edges: Vec<Segment>,
    buckets: Buckets,
    /// For each bucket no edge passes through, which side of the outline it is
    /// on; `None` where edges pass.
    inside: Vec<Option<bool>>,
}

impl Outline {
    pub(crate) fn new(polygon: &Polygon, bucket_size: i32) -> Self {
        let bounds = polygon.bounds();
        let mut buckets = Buckets::new(bounds, bucket_size);
        let edges: Vec<Segment> = polygon.edges().collect();
        for (index, edge) in edges.iter().enumerate() {
            let edge_bounds =
                Rect::around([edge.a, edge.b]).unwrap_or_else(|| unreachable!("two points"));
            buckets.insert(edge_bounds, index);
        }
        let inside = (0..buckets.cell_count())
            .map(|cell| {
                buckets
                    .is_empty(cell)
                    .then(|| polygon.contains(buckets.centre_of(cell)))
            })
            .collect();
        Self {
            polygon: polygon.clone(),
            bounds,
            edges,
            buckets,
            inside,
        }
    }

    /// Whether `shape` lies inside the outline, at least `clearance` from it.
    pub(crate) fn holds(&self, shape: &Shape, clearance: i32) -> bool {
        let reach = shape.bounds().inflated(clearance);
        let near_an_edge = self.buckets.candidates(reach).any(|index| {
            Shape::Capsule {
                segment: self.edges[index],
                radius: 0,
            }
            .is_closer_than(shape, clearance)
        });
        if near_an_edge {
            return false;
        }
        // Clear of every edge, the shape lies wholly on one side: one point
        // decides which.
        let point = match shape {
            Shape::Capsule { segment, .. } => segment.a,
            Shape::Polygon(polygon) => polygon.points()[0],
        };
        if !self.bounds.contains(point) {
            return false;
        }
        self.inside[self.buckets.cell_at(point)].unwrap_or_else(|| self.polygon.contains(point))
    }
}
