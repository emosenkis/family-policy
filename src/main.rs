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
mod time_limits;

use cli::{Args, Commands, ConfigCommands, TimeLimitsCommands};

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
            Commands::Config { command } => {
                match command {
                    ConfigCommands::Init { output, force } => {
                        commands::config::init(output, force, args.verbose)
                    }
                }
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
            Commands::TimeLimits { command } => {
                run_time_limits_command(command, args.verbose)
            }
        };
    }

    // No subcommand: run in local mode (backward compatibility)
    commands::run_local_mode(args)
}

#[tokio::main]
async fn run_time_limits_command(command: TimeLimitsCommands, verbose: bool) -> Result<()> {
    match command {
        TimeLimitsCommands::Init { output, force } => {
            commands::time_limits::init(output, force, verbose)
        }
        TimeLimitsCommands::AddChild { id, name, os_users, weekday_hours, weekend_hours } => {
            commands::time_limits::add_child(id, name, os_users, weekday_hours, weekend_hours, verbose)
        }
        TimeLimitsCommands::StartTracker { no_daemon } => {
            commands::time_limits::start_tracker(no_daemon, verbose).await
        }
        TimeLimitsCommands::StopTracker => {
            commands::time_limits::stop_tracker(verbose)
        }
        TimeLimitsCommands::StatusTracker => {
            commands::time_limits::status_tracker(verbose).await
        }
        TimeLimitsCommands::GrantExtension { child_id, minutes, password, reason } => {
            commands::time_limits::grant_extension(child_id, minutes, password, reason, verbose).await
        }
        TimeLimitsCommands::ResetTime { child_id, password } => {
            commands::time_limits::reset_time(child_id, password, verbose).await
        }
        TimeLimitsCommands::SetPassword { password } => {
            commands::time_limits::set_password(password, verbose)
        }
        TimeLimitsCommands::History { child_id, days } => {
            commands::time_limits::history(child_id, days, verbose)
        }
    }
}
