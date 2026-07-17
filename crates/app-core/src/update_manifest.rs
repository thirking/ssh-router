use std::collections::HashMap;

use semver::Version;
use serde::Deserialize;

use crate::{AppError, AppResult, UpdateError};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default, alias = "notes")]
    pub notes: Option<String>,
    #[serde(default, alias = "pub_date")]
    pub pub_date: Option<String>,
    pub platforms: HashMap<String, PlatformUpdate>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PlatformUpdate {
    pub signature: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCandidate {
    pub current_version: String,
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub signature: String,
    pub url: String,
}

impl UpdateManifest {
    pub fn parse(contents: &[u8]) -> AppResult<Self> {
        serde_json::from_slice(contents).map_err(|error| {
            UpdateError::InvalidManifest {
                message: error.to_string(),
            }
            .into()
        })
    }

    pub fn candidate_for(
        &self,
        current_version: &str,
        platform: &str,
    ) -> AppResult<Option<UpdateCandidate>> {
        let current = parse_version(current_version, "当前版本")?;
        let available = parse_version(&self.version, "更新版本")?;
        if available <= current {
            return Ok(None);
        }
        let platform_update = self.platforms.get(platform).ok_or_else(|| {
            AppError::Update(UpdateError::InvalidManifest {
                message: format!("更新元数据缺少平台: {platform}"),
            })
        })?;
        if platform_update.url.trim().is_empty() || platform_update.signature.trim().is_empty() {
            return Err(UpdateError::InvalidManifest {
                message: "更新地址和签名不能为空".to_string(),
            }
            .into());
        }
        Ok(Some(UpdateCandidate {
            current_version: current_version.to_string(),
            version: self.version.clone(),
            notes: self.notes.clone(),
            pub_date: self.pub_date.clone(),
            signature: platform_update.signature.clone(),
            url: platform_update.url.clone(),
        }))
    }
}

fn parse_version(value: &str, label: &str) -> AppResult<Version> {
    Version::parse(value.trim_start_matches('v')).map_err(|error| {
        UpdateError::InvalidManifest {
            message: format!("{label}无效: {value}: {error}"),
        }
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
      "version": "0.0.9",
      "notes": "性能更新",
      "pub_date": "2026-07-17T00:00:00Z",
      "platforms": {
        "windows-x86_64": {
          "signature": "signature",
          "url": "https://example.test/setup.exe"
        }
      }
    }"#;

    #[test]
    fn parses_tauri_v2_manifest() {
        let manifest = UpdateManifest::parse(MANIFEST.as_bytes()).expect("元数据应可解析");
        let candidate = manifest
            .candidate_for("0.0.8", "windows-x86_64")
            .expect("版本应可比较")
            .expect("应发现更新");
        assert_eq!(candidate.version, "0.0.9");
        assert_eq!(candidate.notes.as_deref(), Some("性能更新"));
    }

    #[test]
    fn current_version_has_no_candidate() {
        let manifest = UpdateManifest::parse(MANIFEST.as_bytes()).expect("元数据应可解析");
        assert_eq!(
            manifest
                .candidate_for("0.0.9", "windows-x86_64")
                .expect("版本应可比较"),
            None
        );
    }

    #[test]
    fn rejects_invalid_manifest() {
        assert!(UpdateManifest::parse(b"not-json").is_err());
    }
}
