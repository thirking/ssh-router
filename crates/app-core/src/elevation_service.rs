use crate::error::AppResult;
use crate::platform::ElevationProvider;

pub struct ElevationService<P> {
    provider: P,
}

impl<P> ElevationService<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> ElevationService<P>
where
    P: ElevationProvider,
{
    pub fn execute(&self, script: &str) -> AppResult<String> {
        self.provider.run_script(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PermissionError;

    struct FakeElevationProvider {
        result: Result<String, PermissionError>,
    }

    impl ElevationProvider for FakeElevationProvider {
        fn run_script(&self, _script: &str) -> AppResult<String> {
            self.result.clone().map_err(Into::into)
        }
    }

    #[test]
    fn returns_success_message() {
        let service = ElevationService::new(FakeElevationProvider {
            result: Ok("OK".to_string()),
        });

        assert_eq!(service.execute("script").expect("执行应成功"), "OK");
    }

    #[test]
    fn preserves_uac_cancellation() {
        let service = ElevationService::new(FakeElevationProvider {
            result: Err(PermissionError::Cancelled),
        });

        let error = service.execute("script").expect_err("取消应返回错误");

        assert!(matches!(
            error,
            crate::error::AppError::Permission(PermissionError::Cancelled)
        ));
    }
}
