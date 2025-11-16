use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current state version
const STATE_VERSION: &str = "1.0";

/// Time limits state tracking
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeLimitsState {
    pub version: String,
    pub state_date: String, // YYYY-MM-DD
    pub children: Vec<ChildState>,
    pub active_session: Option<ActiveSession>,
    pub admin_overrides: Vec<AdminOverride>,
}

impl TimeLimitsState {
    /// Create a new empty state for today
    pub fn new() -> Self {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        Self {
            version: STATE_VERSION.to_string(),
            state_date: today,
            children: Vec::new(),
            active_session: None,
            admin_overrides: Vec::new(),
        }
    }

    /// Check if state needs to be reset (new day)
    pub fn needs_daily_reset(&self) -> bool {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.state_date != today
    }

    /// Reset state for a new day
    pub fn reset_for_new_day(&mut self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.state_date = today;

        // Reset all children's daily usage
        for child in &mut self.children {
            child.today = DayUsage {
                date: self.state_date.clone(),
                used_seconds: 0,
                remaining_seconds: 0, // Will be recalculated
                sessions: Vec::new(),
                warnings_shown: Vec::new(),
                locked_at: None,
            };
        }

        // Clear active session
        self.active_session = None;

        // Clear admin overrides (they don't carry over to new day)
        self.admin_overrides.clear();
    }

    /// Get or create child state
    pub fn get_or_create_child(&mut self, id: &str, name: &str) -> &mut ChildState {
        if let Some(pos) = self.children.iter().position(|c| c.id == id) {
            &mut self.children[pos]
        } else {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            self.children.push(ChildState {
                id: id.to_string(),
                name: name.to_string(),
                today: DayUsage {
                    date: today,
                    used_seconds: 0,
                    remaining_seconds: 0,
                    sessions: Vec::new(),
                    warnings_shown: Vec::new(),
                    locked_at: None,
                },
            });
            self.children.last_mut().unwrap()
        }
    }

    /// Get child state by ID
    pub fn get_child(&self, id: &str) -> Option<&ChildState> {
        self.children.iter().find(|c| c.id == id)
    }

    /// Get mutable child state by ID
    pub fn get_child_mut(&mut self, id: &str) -> Option<&mut ChildState> {
        self.children.iter_mut().find(|c| c.id == id)
    }
}

impl Default for TimeLimitsState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for a single child
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChildState {
    pub id: String,
    pub name: String,
    pub today: DayUsage,
}

/// Daily usage tracking
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DayUsage {
    pub date: String, // YYYY-MM-DD
    pub used_seconds: i64,
    pub remaining_seconds: i64,
    pub sessions: Vec<Session>,
    pub warnings_shown: Vec<String>, // e.g., ["15min", "5min", "1min"]
    pub locked_at: Option<DateTime<Utc>>,
}

impl DayUsage {
    /// Add a completed session
    pub fn add_session(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) {
        let duration = (end - start).num_seconds();
        self.sessions.push(Session {
            start,
            end: Some(end),
            duration_seconds: duration,
        });
        self.used_seconds += duration;
    }

    /// Check if a warning should be shown
    pub fn should_show_warning(&self, threshold_minutes: u32) -> bool {
        let threshold_key = format!("{}min", threshold_minutes);
        !self.warnings_shown.contains(&threshold_key)
    }

    /// Mark a warning as shown
    pub fn mark_warning_shown(&mut self, threshold_minutes: u32) {
        let threshold_key = format!("{}min", threshold_minutes);
        if !self.warnings_shown.contains(&threshold_key) {
            self.warnings_shown.push(threshold_key);
        }
    }

    /// Check if child is currently locked
    pub fn is_locked(&self) -> bool {
        self.locked_at.is_some()
    }

    /// Lock the child
    pub fn lock(&mut self) {
        if self.locked_at.is_none() {
            self.locked_at = Some(Utc::now());
        }
    }

    /// Unlock the child (admin override)
    pub fn unlock(&mut self) {
        self.locked_at = None;
    }
}

/// Session record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Session {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub duration_seconds: i64,
}

/// Active session tracking
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActiveSession {
    pub child_id: String,
    pub session_start: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub paused: bool, // If admin has paused tracking
}

impl ActiveSession {
    /// Create a new active session
    pub fn new(child_id: String) -> Self {
        let now = Utc::now();
        Self {
            child_id,
            session_start: now,
            last_activity: now,
            paused: false,
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Get current session duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        if self.paused {
            0
        } else {
            (Utc::now() - self.session_start).num_seconds()
        }
    }

    /// Check if session is idle (no activity for threshold)
    pub fn is_idle(&self, idle_threshold_seconds: i64) -> bool {
        (Utc::now() - self.last_activity).num_seconds() > idle_threshold_seconds
    }
}

/// Admin override record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminOverride {
    pub child_id: String,
    #[serde(rename = "type")]
    pub override_type: OverrideType,
    pub additional_seconds: Option<i64>,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OverrideType {
    Extension,  // Grant additional time
    Reset,      // Reset time completely
    Unlock,     // Unlock without adding time
    Pause,      // Pause tracking
}

/// Get the platform-specific state file path
pub fn get_state_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/lib/family-policy/time-limits-state.json"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/family-policy/time-limits-state.json",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = PathBuf::from(
            std::env::var("ProgramData")
                .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
        );
        path.push("family-policy");
        path.push("time-limits-state.json");
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported operating system");
    }
}

/// Load state from file
pub fn load_state() -> Result<Option<TimeLimitsState>> {
    let state_path = get_state_path()?;

    if !state_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read state file: {}", state_path.display()))?;

    let mut state: TimeLimitsState = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file: {}", state_path.display()))?;

    // Validate state version
    if state.version != STATE_VERSION {
        eprintln!(
            "Warning: State file version mismatch (expected {}, got {}). Creating new state.",
            STATE_VERSION, state.version
        );
        return Ok(None);
    }

    // Check if we need to reset for a new day
    if state.needs_daily_reset() {
        state.reset_for_new_day();
        save_state(&state)?; // Save the reset state
    }

    Ok(Some(state))
}

/// Save state to file
pub fn save_state(state: &TimeLimitsState) -> Result<()> {
    let state_path = get_state_path()?;

    // Ensure parent directory exists
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
    }

    // Serialize to JSON
    let content = serde_json::to_string_pretty(state)
        .context("Failed to serialize state")?;

    // Write atomically
    crate::platform::common::atomic_write(&state_path, content.as_bytes())
        .with_context(|| format!("Failed to write state file: {}", state_path.display()))?;

    Ok(())
}

/// Delete the state file
pub fn delete_state() -> Result<()> {
    let state_path = get_state_path()?;

    if state_path.exists() {
        std::fs::remove_file(&state_path)
            .with_context(|| format!("Failed to delete state file: {}", state_path.display()))?;
    }

    Ok(())
}

/// Historical usage data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageHistory {
    pub version: String,
    pub records: Vec<DayRecord>,
}

impl UsageHistory {
    pub fn new() -> Self {
        Self {
            version: STATE_VERSION.to_string(),
            records: Vec::new(),
        }
    }

    /// Add a day record
    pub fn add_record(&mut self, record: DayRecord) {
        // Keep records sorted by date, most recent first
        self.records.insert(0, record);

        // Keep only last 90 days
        if self.records.len() > 90 {
            self.records.truncate(90);
        }
    }

    /// Get records for a specific child
    pub fn get_child_records(&self, child_id: &str, days: u32) -> Vec<&ChildDayRecord> {
        self.records
            .iter()
            .take(days as usize)
            .filter_map(|r| r.children.iter().find(|c| c.id == child_id))
            .collect()
    }
}

impl Default for UsageHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Day record in history
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DayRecord {
    pub date: String,
    pub children: Vec<ChildDayRecord>,
}

/// Child's usage for a specific day
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChildDayRecord {
    pub id: String,
    pub name: String,
    pub used_seconds: i64,
    pub limit_seconds: i64,
    pub sessions_count: usize,
    pub overrides: Vec<AdminOverride>,
}

/// Get the platform-specific history file path
pub fn get_history_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/lib/family-policy/time-limits-history.json"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/family-policy/time-limits-history.json",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let mut path = PathBuf::from(
            std::env::var("ProgramData")
                .unwrap_or_else(|_| "C:\\ProgramData".to_string()),
        );
        path.push("family-policy");
        path.push("time-limits-history.json");
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported operating system");
    }
}

/// Load history from file
pub fn load_history() -> Result<UsageHistory> {
    let history_path = get_history_path()?;

    if !history_path.exists() {
        return Ok(UsageHistory::new());
    }

    let content = std::fs::read_to_string(&history_path)
        .with_context(|| format!("Failed to read history file: {}", history_path.display()))?;

    let history: UsageHistory = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse history file: {}", history_path.display()))?;

    Ok(history)
}

/// Save history to file
pub fn save_history(history: &UsageHistory) -> Result<()> {
    let history_path = get_history_path()?;

    // Ensure parent directory exists
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create history directory: {}", parent.display()))?;
    }

    // Serialize to JSON
    let content = serde_json::to_string_pretty(history)
        .context("Failed to serialize history")?;

    // Write atomically
    crate::platform::common::atomic_write(&history_path, content.as_bytes())
        .with_context(|| format!("Failed to write history file: {}", history_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_has_current_date() {
        let state = TimeLimitsState::new();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(state.state_date, today);
    }

    #[test]
    fn test_needs_daily_reset() {
        let mut state = TimeLimitsState::new();
        assert!(!state.needs_daily_reset());

        // Simulate old date
        state.state_date = "2020-01-01".to_string();
        assert!(state.needs_daily_reset());
    }

    #[test]
    fn test_get_or_create_child() {
        let mut state = TimeLimitsState::new();

        let child1 = state.get_or_create_child("kid1", "Alice");
        assert_eq!(child1.id, "kid1");
        assert_eq!(child1.name, "Alice");

        // Should return existing child
        let child1_again = state.get_or_create_child("kid1", "Alice");
        assert_eq!(child1_again.id, "kid1");

        assert_eq!(state.children.len(), 1);
    }

    #[test]
    fn test_day_usage_warnings() {
        let mut usage = DayUsage {
            date: "2025-11-16".to_string(),
            used_seconds: 0,
            remaining_seconds: 3600,
            sessions: vec![],
            warnings_shown: vec![],
            locked_at: None,
        };

        assert!(usage.should_show_warning(15));
        usage.mark_warning_shown(15);
        assert!(!usage.should_show_warning(15));
        assert!(usage.should_show_warning(5));
    }

    #[test]
    fn test_day_usage_lock() {
        let mut usage = DayUsage {
            date: "2025-11-16".to_string(),
            used_seconds: 0,
            remaining_seconds: 0,
            sessions: vec![],
            warnings_shown: vec![],
            locked_at: None,
        };

        assert!(!usage.is_locked());
        usage.lock();
        assert!(usage.is_locked());
        usage.unlock();
        assert!(!usage.is_locked());
    }

    #[test]
    fn test_active_session_duration() {
        let session = ActiveSession::new("kid1".to_string());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(session.duration_seconds() >= 1);
    }

    #[test]
    fn test_active_session_paused() {
        let mut session = ActiveSession::new("kid1".to_string());
        session.paused = true;
        assert_eq!(session.duration_seconds(), 0);
    }

    #[test]
    fn test_usage_history_add_record() {
        let mut history = UsageHistory::new();

        history.add_record(DayRecord {
            date: "2025-11-16".to_string(),
            children: vec![],
        });

        assert_eq!(history.records.len(), 1);
    }

    #[test]
    fn test_usage_history_limits_to_90_days() {
        let mut history = UsageHistory::new();

        for i in 0..100 {
            history.add_record(DayRecord {
                date: format!("2025-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1),
                children: vec![],
            });
        }

        assert_eq!(history.records.len(), 90);
    }
}
