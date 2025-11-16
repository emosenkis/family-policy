/// Time limits module for managing kids' computer usage
///
/// This module provides functionality to:
/// - Configure time limits per child
/// - Track computer usage in real-time
/// - Enforce limits by locking the computer
/// - Support admin overrides
/// - Handle both shared and individual login scenarios

pub mod config;
pub mod state;
pub mod tracker;
pub mod enforcement;
pub mod scheduler;
pub mod auth;
pub mod platform;

pub use config::{TimeLimitsConfig, ChildProfile, TimeLimit, LockAction};
pub use state::{TimeLimitsState, ChildState, DayUsage, Session, AdminOverride};
pub use tracker::TimeTracker;
pub use enforcement::LockEnforcer;
pub use scheduler::ScheduleCalculator;
pub use auth::AdminAuth;
