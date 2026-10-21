mod bench;
mod corpus;
mod e2e;
mod kicad;
mod layers;
mod negotiation;
mod running_kicad;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("layers") => layers::check(),
        Some("e2e") => e2e::run(&args[1..]),
        Some("bench") => bench::run(&args[1..]),
        Some("corpus") => corpus::run(&args[1..]),
        Some("negotiation") => negotiation::run(&args[1..]),
        _ => bail!(
            "usage: cargo xtask layers | e2e [--local] [RECIPE…] | bench [--local] [--freerouting CMD] [--timeout S] [RECIPE…] | corpus [--timeout S] [--freerouting CMD] [--shard K/N] [--list FILE] | negotiation BOARD"
        ),
    }
}
