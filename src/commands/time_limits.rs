use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::time_limits::config::{
    TimeLimitsConfig, AdminConfig, ChildProfile, TimeLimitSchedule,
    TimeLimit, SharedLoginConfig, EnforcementConfig,
    load_config, save_config, get_config_path, EXAMPLE_CONFIG,
};
use crate::time_limits::state::{load_state, save_state, load_history};
use crate::time_limits::auth::AdminAuth;
use crate::time_limits::scheduler::ScheduleCalculator;

/// Initialize a new time limits configuration file
pub fn init(output: Option<PathBuf>, force: bool, _verbose: bool) -> Result<()> {
    let output_path = output.unwrap_or_else(|| {
        get_config_path().unwrap_or_else(|_| PathBuf::from("time-limits-config.yaml"))
    });

    // Check if file exists
    if output_path.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse --force to overwrite",
            output_path.display()
        );
    }

    // Write example config
    std::fs::write(&output_path, EXAMPLE_CONFIG)
        .with_context(|| format!("Failed to write config file: {}", output_path.display()))?;

    println!("✓ Created time limits configuration file: {}", output_path.display());
    println!("\nEdit this file to configure time limits for your children.");
    println!("See the comments in the file for detailed configuration options.");
    println!("\nTo enable time tracking, add this to your agent config (agent.conf):");
    println!("\n[time_limits]");
    println!("enabled = true");
    println!("\nThen start the agent: sudo family-policy start");

    Ok(())
}

/// Add a child profile to the configuration
pub fn add_child(
    id: String,
    name: String,
    os_users: Option<String>,
    weekday_hours: u32,
    weekend_hours: u32,
    _verbose: bool,
) -> Result<()> {
    let config_path = get_config_path()?;

    // Load existing config or create new one
    let mut config = if config_path.exists() {
        load_config(&config_path)?
    } else {
        // Create default config
        let password_hash = AdminAuth::hash_password("admin")?;
        TimeLimitsConfig {
            admin: AdminConfig {
                password_hash,
                admin_accounts: vec!["admin".to_string()],
            },
            children: vec![],
            shared_login: SharedLoginConfig::default(),
            enforcement: EnforcementConfig::default(),
        }
    };

    // Parse OS users
    let os_users_vec = os_users
        .map(|s| s.split(',').map(|u| u.trim().to_string()).collect())
        .unwrap_or_default();

    // Create child profile
    let child = ChildProfile {
        id: id.clone(),
        name: name.clone(),
        os_users: os_users_vec,
        limits: TimeLimitSchedule {
            weekday: TimeLimit {
                hours: weekday_hours,
                minutes: 0,
            },
            weekend: TimeLimit {
                hours: weekend_hours,
                minutes: 0,
            },
            custom: vec![],
        },
        warnings: vec![15, 5, 1],
        grace_period: 60,
    };

    // Check for duplicate ID
    if config.children.iter().any(|c| c.id == id) {
        anyhow::bail!("Child with ID '{}' already exists", id);
    }

    // Add child
    config.children.push(child);

    // Save config
    save_config(&config_path, &config)?;

    println!("✓ Added child: {} ({})", name, id);
    println!("  Weekday limit: {} hours", weekday_hours);
    println!("  Weekend limit: {} hours", weekend_hours);
    println!("\nTime tracking will automatically start when the agent runs.");
    println!("Start the agent: sudo family-policy start");

    Ok(())
}

/// Show current time limits status
pub async fn status(_verbose: bool) -> Result<()> {
    let state = load_state()?.context("No active time limits state found.\nTime tracking may not be running.\nEnsure the agent is started with time_limits enabled.")?;
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;

    println!("\n=== Time Limits Status ===\n");

    if let Some(session) = &state.active_session {
        println!("Active Session:");
        if let Some(child_state) = state.get_child(&session.child_id) {
            println!("  Child: {} ({})", child_state.name, child_state.id);
            println!("  Session started: {}", session.session_start.format("%H:%M:%S"));
            println!("  Last activity: {}", session.last_activity.format("%H:%M:%S"));
            println!("  Paused: {}", session.paused);
        }
        println!();
    } else {
        println!("No active session\n");
    }

    println!("Children:");
    for child_state in &state.children {
        // Find child config
        let child_config = config.children.iter().find(|c| c.id == child_state.id);

        println!("  {} ({}):", child_state.name, child_state.id);
        println!("    Used today: {} minutes", child_state.today.used_seconds / 60);
        println!("    Remaining: {} minutes", child_state.today.remaining_seconds / 60);

        if let Some(config) = child_config {
            let limit = ScheduleCalculator::get_limit_for_today(config);
            println!("    Today's limit: {} hours {} minutes", limit.hours, limit.minutes);
        }

        if child_state.today.is_locked() {
            println!("    Status: 🔒 LOCKED");
            if let Some(locked_at) = child_state.today.locked_at {
                println!("    Locked at: {}", locked_at.format("%H:%M:%S"));
            }
        } else {
            println!("    Status: ✓ Active");
        }

        println!();
    }

    if !state.admin_overrides.is_empty() {
        println!("Admin Overrides Today:");
        for override_rec in &state.admin_overrides {
            if let Some(child_state) = state.get_child(&override_rec.child_id) {
                println!("  {} ({}):", child_state.name, override_rec.child_id);
                println!("    Type: {:?}", override_rec.override_type);
                if let Some(seconds) = override_rec.additional_seconds {
                    println!("    Additional time: {} minutes", seconds / 60);
                }
                println!("    Granted by: {} at {}", override_rec.granted_by, override_rec.granted_at.format("%H:%M:%S"));
                if let Some(reason) = &override_rec.reason {
                    println!("    Reason: {}", reason);
                }
            }
        }
        println!();
    }

    Ok(())
}

/// Grant time extension to a child
pub async fn grant_extension(
    child_id: String,
    minutes: u32,
    password: String,
    reason: Option<String>,
    _verbose: bool,
) -> Result<()> {
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;

    // Verify password
    if !AdminAuth::verify_password(&password, &config.admin.password_hash)? {
        anyhow::bail!("Invalid admin password");
    }

    // Load state
    let mut state = load_state()?.context("No active time limits state found")?;

    // Find child
    let child = config.children.iter()
        .find(|c| c.id == child_id)
        .context("Child not found")?;

    // Add override
    state.admin_overrides.push(crate::time_limits::state::AdminOverride {
        child_id: child_id.clone(),
        override_type: crate::time_limits::state::OverrideType::Extension,
        additional_seconds: Some((minutes as i64) * 60),
        granted_at: chrono::Utc::now(),
        granted_by: AdminAuth::get_current_username()?,
        reason: reason.clone(),
    });

    // Unlock if locked
    if let Some(child_state) = state.get_child_mut(&child_id) {
        child_state.today.unlock();
    }

    // Save state
    save_state(&state)?;

    println!("✓ Granted {} minute extension to {}", minutes, child.name);
    if let Some(r) = reason {
        println!("  Reason: {}", r);
    }

    Ok(())
}

/// Reset a child's time for today
pub async fn reset_time(child_id: String, password: String, _verbose: bool) -> Result<()> {
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;

    // Verify password
    if !AdminAuth::verify_password(&password, &config.admin.password_hash)? {
        anyhow::bail!("Invalid admin password");
    }

    // Load state
    let mut state = load_state()?.context("No active time limits state found")?;

    // Find child
    let child = config.children.iter()
        .find(|c| c.id == child_id)
        .context("Child not found")?;

    // Reset time
    if let Some(child_state) = state.get_child_mut(&child_id) {
        child_state.today.used_seconds = 0;
        child_state.today.sessions.clear();
        child_state.today.warnings_shown.clear();
        child_state.today.unlock();

        // Add override record
        state.admin_overrides.push(crate::time_limits::state::AdminOverride {
            child_id: child_id.clone(),
            override_type: crate::time_limits::state::OverrideType::Reset,
            additional_seconds: None,
            granted_at: chrono::Utc::now(),
            granted_by: AdminAuth::get_current_username()?,
            reason: Some("Time reset".to_string()),
        });

        // Save state
        save_state(&state)?;

        println!("✓ Reset time for {}", child.name);
    } else {
        anyhow::bail!("Child state not found");
    }

    Ok(())
}

/// Set admin password
pub fn set_password(password: String, _verbose: bool) -> Result<()> {
    let config_path = get_config_path()?;

    // Load config
    let mut config = load_config(&config_path)
        .context("Failed to load time limits configuration")?;

    // Hash password
    let hash = AdminAuth::hash_password(&password)?;

    // Update config
    config.admin.password_hash = hash;

    // Save config
    save_config(&config_path, &config)?;

    println!("✓ Admin password updated");

    Ok(())
}

/// Show usage history for a child
pub fn history(child_id: String, days: u32, _verbose: bool) -> Result<()> {
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;

    // Find child
    let child = config.children.iter()
        .find(|c| c.id == child_id)
        .context("Child not found")?;

    // Load history
    let history = load_history()?;

    println!("\n=== Usage History for {} ===\n", child.name);

    let records = history.get_child_records(&child_id, days);

    if records.is_empty() {
        println!("No usage history found");
        return Ok(());
    }

    for record in records {
        let used_hours = record.used_seconds / 3600;
        let used_minutes = (record.used_seconds % 3600) / 60;
        let limit_hours = record.limit_seconds / 3600;
        let limit_minutes = (record.limit_seconds % 3600) / 60;

        println!("Date: {}", record.id);
        println!("  Used: {}h {}m / {}h {}m", used_hours, used_minutes, limit_hours, limit_minutes);
        println!("  Sessions: {}", record.sessions_count);

        if !record.overrides.is_empty() {
            println!("  Overrides: {}", record.overrides.len());
        }

        println!();
    }

    Ok(())
}
