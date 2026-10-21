use super::sexpr::Sexp;
use crate::BoardError;
use traxide_contracts::geometry::Point;

/// How a Specctra file measures. A design gives lengths in its unit, with
/// decimals; a session gives them as whole resolution steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Units {
    nm_per_unit: i64,
    steps_per_unit: i64,
}

impl Units {
    /// From a design's `(unit …)` and `(resolution unit steps)`; the resolution's
    /// unit applies when there is no `(unit …)`.
    pub(crate) fn of_design(root: &Sexp) -> Result<Self, BoardError> {
        let resolution = Self::of_session(root.required_child("resolution")?)?;
        let nm_per_unit = match root.child("unit") {
            Some(unit) => nm_per(unit, unit.arg_atom(0, "unit")?)?,
            None => resolution.nm_per_unit,
        };
        Ok(Self {
            nm_per_unit,
            steps_per_unit: resolution.steps_per_unit * nm_per_unit / resolution.nm_per_unit,
        })
    }

    /// From `(resolution unit steps)`, where lengths are counted in steps.
    pub(crate) fn of_session(resolution: &Sexp) -> Result<Self, BoardError> {
        let nm_per_unit = nm_per(resolution, resolution.arg_atom(0, "resolution unit")?)?;
        let steps_per_unit = resolution
            .arg_atom(1, "resolution")?
            .parse()
            .ok()
            .filter(|&s: &i64| s > 0)
            .ok_or_else(|| resolution.invalid("resolution must be a positive integer"))?;
        Ok(Self {
            nm_per_unit,
            steps_per_unit,
        })
    }

    /// The units in which a design with steps of `resolution_nm` is written back.
    pub(crate) fn for_resolution(resolution_nm: i32) -> Self {
        let steps_per_um = 1_000 / i64::from(resolution_nm.max(1));
        if steps_per_um >= 1 && steps_per_um * i64::from(resolution_nm) == 1_000 {
            Self {
                nm_per_unit: 1_000,
                steps_per_unit: steps_per_um,
            }
        } else {
            Self {
                nm_per_unit: 1_000,
                steps_per_unit: 1_000,
            }
        }
    }

    pub(crate) fn resolution_nm(&self) -> i32 {
        i32::try_from((self.nm_per_unit / self.steps_per_unit).max(1)).unwrap_or(i32::MAX)
    }

    /// A design's lengths are in its unit; this scale reads them.
    pub(crate) fn design_lengths(self) -> Self {
        Self {
            nm_per_unit: self.nm_per_unit,
            steps_per_unit: 1,
        }
    }

    pub(crate) fn length(&self, sexp: &Sexp) -> Result<i32, BoardError> {
        let text = sexp
            .atom()
            .ok_or_else(|| sexp.invalid("expected a number"))?;
        let nm = decimal_times(text, self.nm_per_unit, self.steps_per_unit)
            .ok_or_else(|| sexp.invalid(format!("not a number: {text}")))?;
        i32::try_from(nm).map_err(|_| sexp.invalid(format!("{text} is too large for a board")))
    }

    pub(crate) fn point(&self, x: &Sexp, y: &Sexp) -> Result<Point, BoardError> {
        let point = Point::new(self.length(x)?, self.length(y)?);
        if point.is_within_bounds() {
            Ok(point)
        } else {
            Err(x.invalid("coordinate beyond ±1 m"))
        }
    }

    pub(crate) fn points(&self, coordinates: &[Sexp]) -> Result<Vec<Point>, BoardError> {
        let (pairs, _odd_one_out) = coordinates.as_chunks::<2>();
        pairs.iter().map(|[x, y]| self.point(x, y)).collect()
    }

    /// `nm` in whole steps, rounded half away from zero.
    pub(crate) fn steps(&self, nm: i32) -> i64 {
        let (numerator, denominator) = (i64::from(nm) * self.steps_per_unit, self.nm_per_unit);
        (numerator + numerator.signum() * denominator / 2) / denominator
    }

    pub(crate) fn resolution_text(&self) -> String {
        format!(
            "(resolution um {})",
            self.steps_per_unit * 1_000 / self.nm_per_unit
        )
    }
}

fn nm_per(at: &Sexp, unit: &str) -> Result<i64, BoardError> {
    match unit.to_ascii_lowercase().as_str() {
        "inch" => Ok(25_400_000),
        "mil" => Ok(25_400),
        "cm" => Ok(10_000_000),
        "mm" => Ok(1_000_000),
        "um" => Ok(1_000),
        _ => Err(at.invalid(format!("unknown unit {unit}"))),
    }
}

/// `text` as a decimal number times `multiplier / divisor`, rounded half away from
/// zero, without going through floating point, which would round exact lengths.
fn decimal_times(text: &str, multiplier: i64, divisor: i64) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty())
        || !all_digits(whole)
        || !all_digits(fraction)
        || fraction.len() > 18
    {
        return None;
    }
    let denominator = 10_i128.pow(u32::try_from(fraction.len()).ok()?) * i128::from(divisor);
    let numerator = format!("{whole}{fraction}").parse::<i128>().ok()? * i128::from(multiplier);
    let rounded = (numerator + denominator / 2) / denominator;
    i64::try_from(if negative { -rounded } else { rounded }).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_scale_exactly() {
        assert_eq!(decimal_times("200.1", 1_000, 1), Some(200_100));
        assert_eq!(decimal_times("-5000.000000", 1_000, 1), Some(-5_000_000));
        assert_eq!(decimal_times("0.0001", 1_000, 1), Some(0));
        assert_eq!(decimal_times("0.0005", 1_000, 1), Some(1));
        assert_eq!(decimal_times("-0.0005", 1_000, 1), Some(-1));
        assert_eq!(decimal_times("1.5", 25_400, 1), Some(38_100));
        assert_eq!(decimal_times("40000", 1_000, 10), Some(4_000_000));
    }

    #[test]
    fn rejects_what_is_not_a_decimal() {
        for text in ["12a", "", ".", "1e3"] {
            assert_eq!(decimal_times(text, 1, 1), None, "{text}");
        }
    }

    #[test]
    fn steps_round_half_away_from_zero() {
        let units = Units::for_resolution(100);
        assert_eq!(units.steps(4_000_049), 40_000);
        assert_eq!(units.steps(4_000_050), 40_001);
        assert_eq!(units.steps(-4_000_050), -40_001);
    }

    #[test]
    fn resolutions_that_do_not_divide_a_micrometre_are_written_in_nanometres() {
        let units = Units::for_resolution(254);
        assert_eq!(units.resolution_text(), "(resolution um 1000)");
        assert_eq!(units.steps(1_234), 1_234);
    }
}
