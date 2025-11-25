pub mod apply;
pub mod diff;
pub mod privileges;

// Re-export commonly used items
pub use apply::{apply_policies_from_config, remove_all_policies, ApplyResult, RemovalResult};
pub use diff::{generate_diff, PolicyDiff, BrowserDiff, ExtensionDiff};
pub use privileges::{check_privileges, is_admin, PrivilegeCheck, PrivilegeLevel};
