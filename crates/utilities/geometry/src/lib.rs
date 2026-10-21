//! Exact integer geometry for PCB routing.
//!
//! Every coordinate and length is an `i32` of nanometres, as in KiCad. Predicates
//! compute in `i128`, so they are exact and deterministic on every platform, as long
//! as coordinates stay within [`MAX_COORDINATE`].

mod point;
mod polygon;
mod rect;
mod region;
mod segment;
mod shape;
mod transform;

pub use point::{MAX_COORDINATE, Point};
pub use polygon::Polygon;
pub use rect::Rect;
pub use region::Region;
pub use segment::Segment;
pub use shape::Shape;
pub use transform::Transform;
