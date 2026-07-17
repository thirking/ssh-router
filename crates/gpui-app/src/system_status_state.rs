#![allow(dead_code)]

use ssh_router_app_core::{OperationMessage, SystemStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemOperation {
    Refresh,
    InstallCli,
    SetDefaultShell,
    RestartSshd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemLoadState {
    Idle,
    Loading,
    Loaded,
    Error(String),
}

pub struct SystemStatusState {
    pub load_state: SystemLoadState,
    pub status: Option<SystemStatus>,
    pub running: Option<SystemOperation>,
    pub cli_sync_prompt_open: bool,
    cli_sync_prompted: bool,
    pub error: Option<String>,
}

impl Default for SystemStatusState {
    fn default() -> Self {
        Self {
            load_state: SystemLoadState::Idle,
            status: None,
            running: None,
            cli_sync_prompt_open: false,
            cli_sync_prompted: false,
            error: None,
        }
    }
}

impl SystemStatusState {
    pub fn begin_refresh(&mut self) -> bool {
        if self.running.is_some() || self.load_state == SystemLoadState::Loading {
            return false;
        }
        self.load_state = SystemLoadState::Loading;
        self.error = None;
        true
    }

    pub fn apply_refresh(&mut self, result: Result<SystemStatus, impl Into<String>>) {
        match result {
            Ok(status) => {
                self.status = Some(status);
                self.load_state = SystemLoadState::Loaded;
                self.error = None;
            }
            Err(error) => {
                let message = error.into();
                self.load_state = SystemLoadState::Error(message.clone());
                self.error = Some(message);
            }
        }
    }

    pub fn begin_operation(&mut self, operation: SystemOperation) -> bool {
        if self.running.is_some() || self.load_state == SystemLoadState::Loading {
            return false;
        }
        self.running = Some(operation);
        self.error = None;
        true
    }

    pub fn apply_operation(&mut self, result: Result<OperationMessage, impl Into<String>>) -> bool {
        self.running = None;
        match result {
            Ok(_) => {
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(error.into());
                false
            }
        }
    }

    pub fn maybe_prompt_cli_sync(&mut self) -> bool {
        let should_prompt = self
            .status
            .as_ref()
            .is_some_and(|status| status.cli_deployed && !status.cli_up_to_date);
        if should_prompt && !self.cli_sync_prompted {
            self.cli_sync_prompted = true;
            self.cli_sync_prompt_open = true;
            return true;
        }
        false
    }

    pub fn close_cli_prompt(&mut self) {
        self.cli_sync_prompt_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(cli_deployed: bool, cli_up_to_date: bool) -> SystemStatus {
        SystemStatus {
            cli_deployed,
            cli_up_to_date,
            cli_path: "cli.exe".to_string(),
            default_shell_set: false,
            default_shell: String::new(),
            config_exists: true,
            sshd_running: true,
            sshd_detail: "Running".to_string(),
        }
    }

    #[test]
    fn refresh_is_single_flight() {
        let mut state = SystemStatusState::default();
        assert!(state.begin_refresh());
        assert!(!state.begin_refresh());
    }

    #[test]
    fn operation_is_disabled_while_refreshing() {
        let mut state = SystemStatusState::default();
        state.begin_refresh();
        assert!(!state.begin_operation(SystemOperation::InstallCli));
    }

    #[test]
    fn cli_prompt_is_shown_once() {
        let mut state = SystemStatusState::default();
        state.apply_refresh(Ok::<SystemStatus, String>(status(true, false)));
        assert!(state.maybe_prompt_cli_sync());
        state.close_cli_prompt();
        assert!(!state.maybe_prompt_cli_sync());
    }

    #[test]
    fn refresh_failure_is_retryable() {
        let mut state = SystemStatusState::default();
        state.begin_refresh();
        state.apply_refresh(Err::<SystemStatus, _>("注册表不可用"));

        assert!(matches!(state.load_state, SystemLoadState::Error(_)));
        assert!(state.begin_refresh());
    }

    #[test]
    fn failed_operation_clears_busy_state() {
        let mut state = SystemStatusState::default();
        assert!(state.begin_operation(SystemOperation::RestartSshd));

        let success = state.apply_operation(Err::<OperationMessage, _>("用户取消"));

        assert!(!success);
        assert_eq!(state.running, None);
        assert_eq!(state.error.as_deref(), Some("用户取消"));
    }

    #[test]
    fn successful_operation_allows_followup_refresh() {
        let mut state = SystemStatusState::default();
        state.begin_operation(SystemOperation::InstallCli);

        assert!(
            state.apply_operation(Ok::<OperationMessage, String>(OperationMessage {
                message: "完成".to_string(),
            }))
        );
        assert!(state.begin_refresh());
    }
}
