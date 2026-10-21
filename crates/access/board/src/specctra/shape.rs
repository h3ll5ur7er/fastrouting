use super::sexpr::Sexp;
use super::units::Units;
use crate::BoardError;
use traxide_contracts::LayerId;
use traxide_contracts::geometry::{Point, Polygon, Rect, Segment, Shape};

/// A shape on the layers it covers, in Specctra's coordinates.
pub(crate) struct LayeredShape {
    pub(crate) layers: Vec<LayerId>,
    pub(crate) shape: Shape,
}

/// A `(circle …)`, `(rect …)`, `(polygon …)` or `(path …)` of a design or a
/// session, measured in `units`, on the layers `layers_named` finds.
pub(crate) fn layered_shape(
    sexp: &Sexp,
    units: &Units,
    layers_named: impl Fn(&Sexp) -> Result<Vec<LayerId>, BoardError>,
) -> Result<LayeredShape, BoardError> {
    let args = sexp.args();
    let layer = args.first().ok_or_else(|| sexp.invalid("missing layer"))?;
    let number = |i: usize| args.get(i).ok_or_else(|| sexp.invalid("missing number"));
    let shape = match sexp.keyword() {
        Some("circle") => {
            let center = match (args.get(2), args.get(3)) {
                (Some(x), Some(y)) => units.point(x, y)?,
                _ => Point::default(),
            };
            Shape::circle(center, units.length(number(1)?)?)
        }
        Some("rect") => {
            let corners = units.points(&args[1..])?;
            Shape::Polygon(Polygon::rectangle(
                Rect::around(corners).ok_or_else(|| sexp.invalid("empty rect"))?,
            ))
        }
        Some("polygon") => {
            let points = distinct(units.points(&args[2..])?);
            // Some tools draw a slot or a thin pad as a polygon of one or two
            // points, which the aperture's width makes a line or a dot.
            match points.as_slice() {
                [a] => Shape::circle(*a, units.length(number(1)?)?.max(1)),
                [a, b] => Shape::track(Segment::new(*a, *b), units.length(number(1)?)?.max(1)),
                _ => Shape::Polygon(polygon(sexp, units, &args[2..])?),
            }
        }
        Some("path") => {
            let width = units.length(number(1)?)?;
            match units.points(&args[2..])?.as_slice() {
                [a] => Shape::circle(*a, width),
                [a, b] => Shape::track(Segment::new(*a, *b), width),
                _ => {
                    return Err(
                        sexp.invalid("paths of more than two points are not supported as shapes")
                    );
                }
            }
        }
        other => {
            return Err(sexp.invalid(format!("unsupported shape {}", other.unwrap_or("?"))));
        }
    };
    Ok(LayeredShape {
        layers: layers_named(layer)?,
        shape,
    })
}

pub(crate) fn polygon(
    sexp: &Sexp,
    units: &Units,
    coordinates: &[Sexp],
) -> Result<Polygon, BoardError> {
    Polygon::new(distinct(units.points(coordinates)?))
        .ok_or_else(|| sexp.invalid("a polygon needs at least three points"))
}

/// The points of a closed shape without repeats: a point written twice in a
/// row, or the first point written again to close the shape.
fn distinct(mut points: Vec<Point>) -> Vec<Point> {
    points.dedup();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}
