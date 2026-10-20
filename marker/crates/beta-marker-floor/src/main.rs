//! Standalone JSON-in / JSON-out CLI; redirect output to a new file after success.
use std::io::{Read, Write};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}
fn run() -> anyhow::Result<()> {
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--help") {
        println!(
            "beta-marker-floor [request.json]\nRead a floor command from a file or stdin. Write one JSON response to stdout.\nOperations: validate, template, import, quantities, prepare, advance, dashboard, damage, schedule, recut_advance, issue, export.\nSee README.md for the versioned request and export contracts."
        );
        return Ok(());
    }
    let mut input = String::new();
    let reader: Box<dyn Read> = if let Some(path) = arg {
        Box::new(std::fs::File::open(path)?)
    } else {
        Box::new(std::io::stdin())
    };
    reader.take(50_000_001).read_to_string(&mut input)?;
    let out = beta_marker_floor::dispatch(&input)?;
    std::io::stdout().write_all(out.as_bytes())?;
    Ok(())
}
