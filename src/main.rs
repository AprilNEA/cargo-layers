//! Entry point for the `cargo layers` subcommand.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

/// Cargo runs `cargo-layers layers <arguments>` for `cargo layers <arguments>`.
#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    Layers(Layers),
}

/// Check each workspace package against the rules in its `[package.metadata.layers]` table.
///
/// Exits with 0 when every rule holds, 1 when a reference breaks a rule, and 2 when the check
/// cannot run.
#[derive(clap::Args)]
#[command(version)]
struct Layers {
    /// `Cargo.toml` of the workspace or package to check.
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,
}

fn main() -> ExitCode {
    let Cargo::Layers(arguments) = Cargo::parse();
    match cargo_layers::check(arguments.manifest_path.as_deref()) {
        Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
        Ok(violations) => {
            for violation in &violations {
                eprintln!("{violation}");
            }
            eprintln!("error: {} module layer violation(s)", violations.len());
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("error: {:#}", anyhow::Error::from(error));
            ExitCode::from(2)
        }
    }
}
