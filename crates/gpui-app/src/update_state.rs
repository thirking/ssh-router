use ssh_router_app_core::UpdateCandidate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdatePhase {
    Idle,
    Checking,
    Available,
    Downloading,
    Installing,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateCheckMode {
    Automatic,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateInstallDecision {
    Started,
    NeedsConfirmation,
    Busy,
    Unavailable,
}

pub struct UpdateState {
    pub phase: UpdatePhase,
    pub dialog_open: bool,
    pub current_version: String,
    pub candidate: Option<UpdateCandidate>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub last_checked_at: Option<std::time::SystemTime>,
    pub error: Option<String>,
    pub confirm_dirty_install: bool,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            dialog_open: false,
            current_version: String::new(),
            candidate: None,
            downloaded_bytes: 0,
            total_bytes: None,
            last_checked_at: None,
            error: None,
            confirm_dirty_install: false,
        }
    }
}

impl UpdateState {
    pub fn begin_check(&mut self) -> bool {
        if matches!(
            self.phase,
            UpdatePhase::Checking | UpdatePhase::Downloading | UpdatePhase::Installing
        ) {
            return false;
        }
        self.phase = UpdatePhase::Checking;
        self.error = None;
        true
    }

    pub fn apply_check(
        &mut self,
        mode: UpdateCheckMode,
        result: Result<Option<UpdateCandidate>, impl Into<String>>,
    ) {
        self.last_checked_at = Some(std::time::SystemTime::now());
        match result {
            Ok(Some(candidate)) => {
                self.current_version = candidate.current_version.clone();
                self.candidate = Some(candidate);
                self.dialog_open = true;
                self.phase = UpdatePhase::Available;
                self.error = None;
            }
            Ok(None) => {
                self.candidate = None;
                self.dialog_open = false;
                self.phase = UpdatePhase::Idle;
                self.error = None;
            }
            Err(error) => {
                if mode == UpdateCheckMode::Manual {
                    self.phase = UpdatePhase::Error;
                    self.error = Some(error.into());
                } else {
                    self.phase = UpdatePhase::Idle;
                    self.error = None;
                }
            }
        }
    }

    pub fn request_download(&mut self, has_unsaved_config: bool) -> UpdateInstallDecision {
        if self.candidate.is_none() {
            return UpdateInstallDecision::Unavailable;
        }
        if matches!(
            self.phase,
            UpdatePhase::Downloading | UpdatePhase::Installing
        ) {
            return UpdateInstallDecision::Busy;
        }
        if has_unsaved_config {
            self.confirm_dirty_install = true;
            self.dialog_open = true;
            return UpdateInstallDecision::NeedsConfirmation;
        }
        self.begin_download();
        UpdateInstallDecision::Started
    }

    pub fn confirm_download(&mut self) -> bool {
        if !self.confirm_dirty_install
            || self.candidate.is_none()
            || matches!(
                self.phase,
                UpdatePhase::Downloading | UpdatePhase::Installing
            )
        {
            return false;
        }
        self.confirm_dirty_install = false;
        self.begin_download();
        true
    }

    fn begin_download(&mut self) {
        self.phase = UpdatePhase::Downloading;
        self.dialog_open = true;
        self.downloaded_bytes = 0;
        self.total_bytes = None;
        self.error = None;
    }

    pub fn progress(&mut self, downloaded_bytes: u64, total_bytes: Option<u64>) {
        self.downloaded_bytes = downloaded_bytes;
        self.total_bytes = total_bytes;
    }

    pub fn finished(&mut self) {
        self.phase = UpdatePhase::Installing;
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.phase = UpdatePhase::Error;
        self.dialog_open = true;
        self.error = Some(message.into());
    }

    pub fn dismiss(&mut self) {
        self.dialog_open = false;
        self.confirm_dirty_install = false;
    }

    pub fn show_available(&mut self) {
        if self.candidate.is_some() {
            self.dialog_open = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> UpdateCandidate {
        UpdateCandidate {
            current_version: "0.0.8".to_string(),
            version: "0.0.9".to_string(),
            notes: None,
            pub_date: None,
            signature: "signature".to_string(),
            url: "https://example.test/setup.exe".to_string(),
        }
    }

    #[test]
    fn update_state_covers_available_download_install() {
        let mut state = UpdateState::default();
        assert!(state.begin_check());
        state.apply_check(
            UpdateCheckMode::Manual,
            Ok::<Option<UpdateCandidate>, String>(Some(candidate())),
        );
        assert_eq!(state.phase, UpdatePhase::Available);
        assert_eq!(
            state.request_download(false),
            UpdateInstallDecision::Started
        );
        state.progress(25, Some(100));
        state.finished();
        assert_eq!(state.phase, UpdatePhase::Installing);
        assert_eq!(state.downloaded_bytes, 25);
    }

    #[test]
    fn update_check_error_is_visible() {
        let mut state = UpdateState::default();
        state.begin_check();
        state.apply_check(
            UpdateCheckMode::Manual,
            Err::<Option<UpdateCandidate>, _>("签名服务不可用"),
        );
        assert_eq!(state.phase, UpdatePhase::Error);
        assert_eq!(state.error.as_deref(), Some("签名服务不可用"));
    }

    #[test]
    fn automatic_check_error_stays_silent() {
        let mut state = UpdateState::default();
        state.begin_check();

        state.apply_check(
            UpdateCheckMode::Automatic,
            Err::<Option<UpdateCandidate>, _>("网络不可用"),
        );

        assert_eq!(state.phase, UpdatePhase::Idle);
        assert_eq!(state.error, None);
    }

    #[test]
    fn dirty_config_requires_confirmation_before_download() {
        let mut state = UpdateState::default();
        state.apply_check(
            UpdateCheckMode::Manual,
            Ok::<Option<UpdateCandidate>, String>(Some(candidate())),
        );

        assert_eq!(
            state.request_download(true),
            UpdateInstallDecision::NeedsConfirmation
        );
        assert_eq!(state.phase, UpdatePhase::Available);
        assert!(state.confirm_download());
        assert_eq!(state.phase, UpdatePhase::Downloading);
    }

    #[test]
    fn repeated_install_is_rejected_while_busy() {
        let mut state = UpdateState::default();
        state.apply_check(
            UpdateCheckMode::Manual,
            Ok::<Option<UpdateCandidate>, String>(Some(candidate())),
        );
        assert_eq!(
            state.request_download(false),
            UpdateInstallDecision::Started
        );

        assert_eq!(state.request_download(false), UpdateInstallDecision::Busy);
        state.finished();
        assert_eq!(state.request_download(false), UpdateInstallDecision::Busy);
    }
}
