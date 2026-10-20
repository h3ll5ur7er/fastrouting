mod layers;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("layers") => layers::check(),
        _ => bail!("usage: cargo xtask layers"),
    }
}
