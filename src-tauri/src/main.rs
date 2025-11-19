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
mod ui;

use cli::{Args, Commands, ConfigCommands};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    match args.command {
        Some(Commands::Apply) | None => {
            // Apply command (explicit) or no subcommand (default) - run local mode
            commands::run_local_mode(args)
        }
        Some(Commands::Config { command }) => {
            match command {
                ConfigCommands::Init { output, force } => {
                    commands::config::init(output, force, args.verbose)
                }
            }
        }
        Some(Commands::InstallService) => {
            commands::agent::install_service(args.verbose)
        }
        Some(Commands::UninstallService) => {
            commands::agent::uninstall_service(args.verbose)
        }
        Some(Commands::Start { no_daemon }) => {
            commands::agent::start(no_daemon, args.verbose)
        }
        Some(Commands::Stop) => {
            commands::agent::stop(args.verbose)
        }
        Some(Commands::CheckNow) => {
            commands::agent::check_now(args.dry_run, args.verbose)
        }
        Some(Commands::Status) => {
            commands::agent::status(args.verbose)
        }
        Some(Commands::ShowConfig) => {
            commands::agent::show_config(args.verbose)
        }
        Some(Commands::Ui) => {
            ui::run()
        }
    }
}
