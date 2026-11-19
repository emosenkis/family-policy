use anyhow::Result;
use clap::Parser;

mod agent;
mod browser;
mod cli;
mod commands;
mod config;
mod core;
mod platform;
mod policy;
mod state;
#[cfg(windows)]
mod windows_service;

use cli::{Args, Commands, ConfigCommands};
use core::privileges::{PrivilegeCheck, check_privileges};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands with privilege checking
    match args.command {
        Some(Commands::Apply) | None => {
            // Require admin, but allow dry-run for regular users
            check_privileges(PrivilegeCheck::admin_or_dry_run(), args.dry_run)?;
            commands::run_local_mode(args)
        }
        Some(Commands::Config { command }) => {
            // Config init doesn't require admin
            check_privileges(PrivilegeCheck::user(), false)?;
            match command {
                ConfigCommands::Init { output, force } => {
                    commands::config::init(output, force, args.verbose)
                }
            }
        }
        Some(Commands::Daemon) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::daemon(args.verbose)
        }
        Some(Commands::InstallService) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::install_service(args.verbose)
        }
        Some(Commands::UninstallService) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::uninstall_service(args.verbose)
        }
        Some(Commands::Start { no_daemon }) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::start(no_daemon, args.verbose)
        }
        Some(Commands::Stop) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::stop(args.verbose)
        }
        Some(Commands::CheckNow) => {
            check_privileges(PrivilegeCheck::admin_or_dry_run(), args.dry_run)?;
            commands::agent::check_now(args.dry_run, args.verbose)
        }
        Some(Commands::Status) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            commands::agent::status(args.verbose)
        }
        Some(Commands::ShowConfig) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            commands::agent::show_config(args.verbose)
        }
        Some(Commands::Setup {
            url,
            token,
            poll_interval,
            force,
        }) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            commands::agent::setup(url, token, poll_interval, force)
        }
        #[cfg(windows)]
        Some(Commands::Service) => windows_service::run(),
    }
}
