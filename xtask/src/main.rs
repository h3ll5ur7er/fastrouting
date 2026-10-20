mod e2e;
mod layers;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("layers") => layers::check(),
        Some("e2e") => e2e::run(&args[1..]),
        _ => bail!("usage: cargo xtask layers | cargo xtask e2e [--local] [RECIPE…]"),
    }
}
