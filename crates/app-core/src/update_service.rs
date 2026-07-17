use crate::{signature::verify_installer, AppResult, UpdateCandidate, UpdateManifest};

pub struct UpdateService {
    current_version: String,
    platform: String,
}

impl UpdateService {
    pub fn new(current_version: impl Into<String>, platform: impl Into<String>) -> Self {
        Self {
            current_version: current_version.into(),
            platform: platform.into(),
        }
    }

    pub fn check_manifest(&self, contents: &[u8]) -> AppResult<Option<UpdateCandidate>> {
        UpdateManifest::parse(contents)?.candidate_for(&self.current_version, &self.platform)
    }

    pub fn verify_download(&self, candidate: &UpdateCandidate, contents: &[u8]) -> AppResult<()> {
        verify_installer(contents, &candidate.signature)
    }
}
