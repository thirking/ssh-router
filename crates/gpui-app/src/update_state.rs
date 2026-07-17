#![allow(dead_code)]

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

pub struct UpdateState {
    pub phase: UpdatePhase,
    pub dialog_open: bool,
    pub current_version: String,
    pub candidate: Option<UpdateCandidate>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub last_checked_at: Option<std::time::SystemTime>,
    pub error: Option<String>,
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

    pub fn apply_check(&mut self, result: Result<Option<UpdateCandidate>, impl Into<String>>) {
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
                self.phase = UpdatePhase::Error;
                self.error = Some(error.into());
            }
        }
    }

    pub fn begin_download(&mut self) -> bool {
        if self.candidate.is_none() || self.phase == UpdatePhase::Downloading {
            return false;
        }
        self.phase = UpdatePhase::Downloading;
        self.dialog_open = true;
        self.downloaded_bytes = 0;
        self.total_bytes = None;
        self.error = None;
        true
    }

    pub fn started(&mut self, total_bytes: Option<u64>) {
        self.total_bytes = total_bytes;
    }

    pub fn progress(&mut self, chunk_bytes: u64) {
        self.downloaded_bytes = self.downloaded_bytes.saturating_add(chunk_bytes);
    }

    pub fn finished(&mut self) {
        self.phase = UpdatePhase::Installing;
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.phase = UpdatePhase::Error;
        self.error = Some(message.into());
    }

    pub fn dismiss(&mut self) {
        self.dialog_open = false;
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
        state.apply_check(Ok::<Option<UpdateCandidate>, String>(Some(candidate())));
        assert_eq!(state.phase, UpdatePhase::Available);
        assert!(state.begin_download());
        state.started(Some(100));
        state.progress(25);
        state.finished();
        assert_eq!(state.phase, UpdatePhase::Installing);
        assert_eq!(state.downloaded_bytes, 25);
    }

    #[test]
    fn update_check_error_is_visible() {
        let mut state = UpdateState::default();
        state.begin_check();
        state.apply_check(Err::<Option<UpdateCandidate>, _>("签名服务不可用"));
        assert_eq!(state.phase, UpdatePhase::Error);
        assert_eq!(state.error.as_deref(), Some("签名服务不可用"));
    }
}
