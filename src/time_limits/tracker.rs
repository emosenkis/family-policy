use anyhow::{Context, Result};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{info, warn, error, debug};

use crate::time_limits::config::{TimeLimitsConfig, ChildProfile};
use crate::time_limits::state::{TimeLimitsState, ActiveSession, AdminOverride, OverrideType};
use crate::time_limits::scheduler::ScheduleCalculator;
use crate::time_limits::enforcement::LockEnforcer;
use crate::time_limits::auth::AdminAuth;

/// Time tracker daemon
pub struct TimeTracker {
    config: Arc<RwLock<TimeLimitsConfig>>,
    state: Arc<Mutex<TimeLimitsState>>,
    enforcer: Arc<LockEnforcer>,
    running: Arc<Mutex<bool>>,
    paused: Arc<Mutex<bool>>,
}

impl TimeTracker {
    /// Create a new time tracker
    pub fn new(config: TimeLimitsConfig, state: TimeLimitsState) -> Self {
        let enforcer = LockEnforcer::new(config.clone());

        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(Mutex::new(state)),
            enforcer: Arc::new(enforcer),
            running: Arc::new(Mutex::new(false)),
            paused: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the time tracking loop
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        if *running {
            anyhow::bail!("Time tracker is already running");
        }
        *running = true;
        drop(running);

        info!("Starting time tracker");

        let config = self.config.clone();
        let state = self.state.clone();
        let enforcer = self.enforcer.clone();
        let running = self.running.clone();
        let paused = self.paused.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Check if we should stop
                if !*running.lock().await {
                    info!("Time tracker stopped");
                    break;
                }

                // Check if paused
                if *paused.lock().await {
                    debug!("Time tracker is paused");
                    continue;
                }

                // Run tracking iteration
                if let Err(e) = Self::track_iteration(
                    &config,
                    &state,
                    &enforcer,
                ).await {
                    error!("Error in tracking iteration: {:#}", e);
                }
            }
        });

        Ok(())
    }

    /// Stop the time tracking loop
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        info!("Stopping time tracker");
        Ok(())
    }

    /// Pause time tracking (admin override)
    pub async fn pause(&self) -> Result<()> {
        let mut paused = self.paused.lock().await;
        *paused = true;

        let mut state = self.state.lock().await;
        if let Some(session) = &mut state.active_session {
            session.paused = true;
        }

        info!("Time tracking paused");
        Ok(())
    }

    /// Resume time tracking
    pub async fn resume(&self) -> Result<()> {
        let mut paused = self.paused.lock().await;
        *paused = false;

        let mut state = self.state.lock().await;
        if let Some(session) = &mut state.active_session {
            session.paused = false;
        }

        info!("Time tracking resumed");
        Ok(())
    }

    /// Select a child for the current session (shared login mode)
    pub async fn select_child(&self, child_id: &str) -> Result<()> {
        let config = self.config.read().await;
        let mut state = self.state.lock().await;

        // Find the child
        let child = config.children.iter()
            .find(|c| c.id == child_id)
            .context("Child not found")?;

        // Ensure we're in shared login mode
        if !config.shared_login.enabled {
            anyhow::bail!("Shared login mode is not enabled");
        }

        // End current session if any
        let current_child_id = state.active_session.as_ref().map(|s| s.child_id.clone());
        if let Some(current_id) = current_child_id {
            Self::end_session_internal(&mut state, current_id)?;
        }

        // Start new session
        state.active_session = Some(ActiveSession::new(child_id.to_string()));

        // Ensure child state exists
        state.get_or_create_child(&child.id, &child.name);

        info!("Selected child: {} ({})", child.name, child.id);
        Ok(())
    }

    /// Grant time extension to a child (admin override)
    pub async fn grant_extension(
        &self,
        child_id: &str,
        additional_minutes: u32,
        admin_password: &str,
        reason: Option<String>,
    ) -> Result<()> {
        let config = self.config.read().await;

        // Verify admin password
        if !AdminAuth::verify_password(admin_password, &config.admin.password_hash)? {
            anyhow::bail!("Invalid admin password");
        }

        let mut state = self.state.lock().await;

        // Record the override
        let admin_user = AdminAuth::get_current_username()?;
        state.admin_overrides.push(AdminOverride {
            child_id: child_id.to_string(),
            override_type: OverrideType::Extension,
            additional_seconds: Some((additional_minutes as i64) * 60),
            granted_at: Utc::now(),
            granted_by: admin_user,
            reason,
        });

        // Unlock if locked
        if let Some(child) = state.get_child_mut(child_id) {
            child.today.unlock();
        }

        info!("Granted {} minute extension to child: {}", additional_minutes, child_id);
        Ok(())
    }

    /// Reset a child's time for today (admin override)
    pub async fn reset_time(&self, child_id: &str, admin_password: &str) -> Result<()> {
        let config = self.config.read().await;

        // Verify admin password
        if !AdminAuth::verify_password(admin_password, &config.admin.password_hash)? {
            anyhow::bail!("Invalid admin password");
        }

        let mut state = self.state.lock().await;

        // Reset the child's time
        if let Some(child) = state.get_child_mut(child_id) {
            child.today.used_seconds = 0;
            child.today.sessions.clear();
            child.today.warnings_shown.clear();
            child.today.unlock();

            // Record the override
            let admin_user = AdminAuth::get_current_username()?;
            state.admin_overrides.push(AdminOverride {
                child_id: child_id.to_string(),
                override_type: OverrideType::Reset,
                additional_seconds: None,
                granted_at: Utc::now(),
                granted_by: admin_user,
                reason: Some("Time reset".to_string()),
            });

            info!("Reset time for child: {}", child_id);
        } else {
            anyhow::bail!("Child not found: {}", child_id);
        }

        Ok(())
    }

    /// Get current state (for UI)
    pub async fn get_state(&self) -> TimeLimitsState {
        self.state.lock().await.clone()
    }

    /// Get config (for UI)
    pub async fn get_config(&self) -> TimeLimitsConfig {
        self.config.read().await.clone()
    }

    /// Single iteration of time tracking
    async fn track_iteration(
        config: &Arc<RwLock<TimeLimitsConfig>>,
        state: &Arc<Mutex<TimeLimitsState>>,
        enforcer: &Arc<LockEnforcer>,
    ) -> Result<()> {
        let config_read = config.read().await;
        let mut state_write = state.lock().await;

        // Check if we need daily reset
        if state_write.needs_daily_reset() {
            info!("Resetting state for new day");
            state_write.reset_for_new_day();
            crate::time_limits::state::save_state(&state_write)?;
        }

        // Get current active session
        let active_session = match &state_write.active_session {
            Some(session) => session.clone(),
            None => {
                // No active session - try to auto-detect
                Self::auto_detect_child(&config_read, &mut state_write)?;
                return Ok(());
            }
        };

        // Skip if paused
        if active_session.paused {
            return Ok(());
        }

        let child_id = active_session.child_id.clone();

        // Find child config
        let child_config = config_read.children.iter()
            .find(|c| c.id == child_id)
            .context("Active child not found in config")?;

        // Calculate additional seconds from overrides (before mutable borrow)
        let additional_seconds = ScheduleCalculator::get_today_overrides(
            &child_id,
            &state_write.admin_overrides,
        );

        // Get or create child state
        let child_state = state_write.get_or_create_child(&child_id, &child_config.name);

        // Check if already locked
        if child_state.today.is_locked() {
            debug!("Child {} is already locked", child_id);
            return Ok(());
        }

        // Update time used (10 seconds per iteration)
        child_state.today.used_seconds += 10;

        // Calculate remaining time
        let remaining_seconds = ScheduleCalculator::calculate_remaining_time(
            child_config,
            child_state.today.used_seconds,
            Some(additional_seconds),
        );

        child_state.today.remaining_seconds = remaining_seconds;

        // Check for warnings
        for &warning_minutes in &child_config.warnings {
            let warning_seconds = (warning_minutes as i64) * 60;
            if remaining_seconds <= warning_seconds && remaining_seconds > warning_seconds - 10 {
                if child_state.today.should_show_warning(warning_minutes) {
                    enforcer.send_warning(child_state, warning_minutes)?;
                    child_state.today.mark_warning_shown(warning_minutes);
                }
            }
        }

        // Check if time has expired
        if remaining_seconds <= 0 {
            info!("Time expired for child: {}", child_config.name);

            // Show final warning with grace period
            enforcer.send_final_warning(child_state, child_config.grace_period)?;

            // Wait for grace period
            tokio::time::sleep(Duration::from_secs(child_config.grace_period)).await;

            // Lock the child
            child_state.today.lock();

            // Enforce the lock
            enforcer.enforce_lock(child_state)?;
        }

        // Save state
        crate::time_limits::state::save_state(&state_write)?;

        Ok(())
    }

    /// Auto-detect which child is using the computer (individual login mode)
    fn auto_detect_child(
        config: &TimeLimitsConfig,
        state: &mut TimeLimitsState,
    ) -> Result<()> {
        if config.shared_login.enabled {
            // In shared login mode, require explicit selection
            debug!("Shared login mode - waiting for child selection");
            return Ok(());
        }

        // Get current OS username
        let current_user = AdminAuth::get_current_username()?;

        // Check if current user is an admin
        if AdminAuth::is_admin_account(&current_user, &config.admin.admin_accounts) {
            debug!("Current user is admin, not tracking time");
            return Ok(());
        }

        // Find child by OS username
        for child in &config.children {
            if child.os_users.contains(&current_user) {
                info!("Auto-detected child: {} ({})", child.name, child.id);
                state.active_session = Some(ActiveSession::new(child.id.clone()));
                state.get_or_create_child(&child.id, &child.name);
                return Ok(());
            }
        }

        warn!("Current user '{}' is not configured as admin or child", current_user);
        Ok(())
    }

    /// End the current session
    fn end_session_internal(state: &mut TimeLimitsState, child_id: String) -> Result<()> {
        let now = Utc::now();
        let session_start = state.active_session.as_ref().map(|s| s.session_start);

        if let Some(child) = state.get_child_mut(&child_id) {
            if let Some(start) = session_start {
                child.today.add_session(start, now);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_limits::config::{AdminConfig, SharedLoginConfig, EnforcementConfig, TimeLimitSchedule, TimeLimit};

    fn make_test_config() -> TimeLimitsConfig {
        TimeLimitsConfig {
            admin: AdminConfig {
                password_hash: AdminAuth::hash_password("admin123").unwrap(),
                admin_accounts: vec!["admin".to_string()],
            },
            children: vec![ChildProfile {
                id: "kid1".to_string(),
                name: "Alice".to_string(),
                os_users: vec!["alice".to_string()],
                limits: TimeLimitSchedule {
                    weekday: TimeLimit {
                        hours: 2,
                        minutes: 0,
                    },
                    weekend: TimeLimit {
                        hours: 4,
                        minutes: 0,
                    },
                    custom: vec![],
                },
                warnings: vec![15, 5, 1],
                grace_period: 60,
            }],
            shared_login: SharedLoginConfig::default(),
            enforcement: EnforcementConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_tracker_creation() {
        let config = make_test_config();
        let state = TimeLimitsState::new();
        let tracker = TimeTracker::new(config, state);

        let is_running = *tracker.running.lock().await;
        assert!(!is_running);
    }

    #[tokio::test]
    async fn test_pause_and_resume() {
        let config = make_test_config();
        let state = TimeLimitsState::new();
        let tracker = TimeTracker::new(config, state);

        tracker.pause().await.unwrap();
        assert!(*tracker.paused.lock().await);

        tracker.resume().await.unwrap();
        assert!(!*tracker.paused.lock().await);
    }

    #[tokio::test]
    async fn test_grant_extension() {
        let config = make_test_config();
        let mut state = TimeLimitsState::new();
        state.get_or_create_child("kid1", "Alice");

        let tracker = TimeTracker::new(config, state);

        let result = tracker.grant_extension(
            "kid1",
            30,
            "admin123",
            Some("Homework".to_string()),
        ).await;

        assert!(result.is_ok());

        let state = tracker.get_state().await;
        assert_eq!(state.admin_overrides.len(), 1);
        assert_eq!(state.admin_overrides[0].additional_seconds, Some(1800));
    }
}
