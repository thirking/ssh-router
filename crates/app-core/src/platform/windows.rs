use crate::error::AppResult;

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> AppResult<Self> {
        super::ensure_supported("初始化 Windows 平台服务")?;
        Ok(Self)
    }
}
