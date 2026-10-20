//! Compile/test the unregistered CLI module without changing lead-owned files.
#[path = "../../beta-marker-cli/src/cmd/queue.rs"]
mod queue;
use clap::Parser;
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    queue: queue::Args,
}
fn main() -> anyhow::Result<()> {
    queue::run(Cli::parse().queue)
}
