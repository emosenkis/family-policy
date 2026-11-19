use anyhow::{Context, Result};
use std::ffi::OsString;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const NAME: &str = "FamilyPolicyAgent";
define_windows_service!(service_main, main);

pub fn run() -> Result<()> {
    service_dispatcher::start(NAME, service_main)
        .context("Failed to connect to Windows Service Control Manager")
}

fn main(_args: Vec<OsString>) {
    crate::commands::utils::init_logging(false);
    let (stop_tx, mut stop_rx) = tokio::sync::mpsc::unbounded_channel();
    let status = match service_control_handler::register(NAME, move |event| match event {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }) {
        Ok(status) => status,
        Err(error) => {
            tracing::error!("Failed to register service handler: {}", error);
            return;
        }
    };
    let report = |state, exit_code| {
        status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: if state == ServiceState::Running {
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
            } else {
                ServiceControlAccept::empty()
            },
            exit_code,
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
    };
    let result = (|| -> Result<()> {
        let config = crate::agent::AgentConfig::load(&crate::agent::get_agent_config_path()?)?;
        let runtime = tokio::runtime::Runtime::new()?;
        report(ServiceState::Running, ServiceExitCode::Win32(0))?;
        runtime.block_on(async {
            tokio::select! {
                result = crate::agent::run_agent_daemon(config) => result,
                _ = stop_rx.recv() => Ok(()),
            }
        })
    })();
    if let Err(ref error) = result {
        tracing::error!("Service failed: {:#}", error);
    }
    let _ = report(
        ServiceState::Stopped,
        ServiceExitCode::Win32(if result.is_ok() { 0 } else { 1 }),
    );
}
