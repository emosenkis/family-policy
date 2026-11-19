pub mod apply;
pub mod diff;
pub mod privileges;

// Re-export commonly used items
pub use apply::{ApplyResult, RemovalResult, apply_policies_from_config, remove_all_policies};
pub use diff::{BrowserDiff, ExtensionDiff, PolicyDiff, generate_diff};
pub use privileges::{PrivilegeCheck, PrivilegeLevel, check_privileges, is_admin};
