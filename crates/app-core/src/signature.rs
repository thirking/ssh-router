use minisign_verify::{PublicKey, Signature};

use crate::{AppResult, UpdateError};

pub const UPDATE_PUBLIC_KEY: &str = "RWRCIe7UU5heZhJWzr8REXRosR9lKZktyS1xnM5mMmCy0wwdfxUgZLTo";

pub fn verify_installer(contents: &[u8], signature: &str) -> AppResult<()> {
    let public_key = PublicKey::from_base64(UPDATE_PUBLIC_KEY).map_err(|error| {
        UpdateError::InvalidSignature {
            message: format!("内置更新公钥无效: {error}"),
        }
    })?;
    let signature =
        Signature::decode(signature).map_err(|error| UpdateError::InvalidSignature {
            message: format!("更新签名格式无效: {error}"),
        })?;
    public_key
        .verify(contents, &signature, false)
        .map_err(|error| {
            UpdateError::InvalidSignature {
                message: error.to_string(),
            }
            .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_signature() {
        let error = verify_installer(b"installer", "invalid").expect_err("无效签名应被拒绝");
        assert!(matches!(
            error,
            crate::AppError::Update(UpdateError::InvalidSignature { .. })
        ));
    }
}
