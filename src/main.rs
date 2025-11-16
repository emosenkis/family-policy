use anyhow::Result;
use clap::Parser;

mod agent;
mod browser;
mod cli;
mod commands;
mod config;
mod platform;
mod policy;
mod state;

use cli::{Args, Commands};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        return match command {
            Commands::Agent { command } => {
                commands::run_agent_command(command, args.verbose)
            }
        };
    }

    // No subcommand: run in local mode (backward compatibility)
    commands::run_local_mode(args)
}
