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
mod ui;

use cli::{Args, Commands, ConfigCommands};
use core::privileges::{check_privileges, PrivilegeCheck};

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
        Some(Commands::UserUi { systray, window }) => {
            check_privileges(PrivilegeCheck::user(), false)?;
            let systray_mode = systray || !window; // Default to systray if neither specified
            ui::user::run(systray_mode)
        }
        Some(Commands::AdminUi) => {
            check_privileges(PrivilegeCheck::admin(), false)?;
            ui::admin::run()
        }
    }
}
