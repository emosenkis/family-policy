// Agent module for GitHub polling functionality
//
// This module implements remote policy management by polling a GitHub repository
// for policy changes. Agents poll a raw GitHub URL, use ETag for efficiency,
// and automatically apply policies when changes are detected.

mod config;
mod daemon;
mod poller;
mod scheduler;
mod state;

pub use config::{AgentConfig, AgentSettings, GitHubConfig, LoggingConfig, SecurityConfig, TimeLimitsSettings, get_agent_config_path};
pub use daemon::{run_agent_daemon, check_and_apply_once};
pub use poller::{GitHubPoller, PolicyFetchResult};
pub use scheduler::PollingScheduler;
pub use state::AgentState;
