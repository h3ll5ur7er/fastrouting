//! Specctra DSN (designs) and SES (sessions): what KiCad, Freerouting and many
//! other tools exchange with autorouters.
//!
//! Specctra's y axis points up, KiCad's and traxide's down: coordinates are read
//! and transformed in Specctra's space and flipped once, on the way out.

mod dsn;
mod ses;
mod sexpr;
mod units;

pub use dsn::parse_dsn;
pub use ses::{parse_ses, write_ses};
