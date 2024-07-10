mod design_entry;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    DesignEntry {
        #[command(subcommand)]
        command: design_entry::Command,
    },
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::DesignEntry { .. } => "design-entry",
        }
    }

    fn run(&self) -> Result<()> {
        match self {
            Self::DesignEntry { command } => {
                command
                    .run()
                    .with_context(|| format!("`{} {}` failed", self.name(), command.name()))?;
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    Cli::parse().command.run()
}
