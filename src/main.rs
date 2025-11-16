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
            Commands::Setup { url, token, poll_interval } => {
                commands::agent::setup(url, token, poll_interval, args.verbose)
            }
            Commands::InstallService => {
                commands::agent::install_service(args.verbose)
            }
            Commands::UninstallService => {
                commands::agent::uninstall_service(args.verbose)
            }
            Commands::Start { no_daemon } => {
                commands::agent::start(no_daemon, args.verbose)
            }
            Commands::Stop => {
                commands::agent::stop(args.verbose)
            }
            Commands::CheckNow => {
                commands::agent::check_now(args.verbose)
            }
            Commands::Status => {
                commands::agent::status(args.verbose)
            }
            Commands::ShowConfig => {
                commands::agent::show_config(args.verbose)
            }
        };
    }

    // No subcommand: run in local mode (backward compatibility)
    commands::run_local_mode(args)
}
