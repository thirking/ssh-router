pub mod error;
pub mod platform;

pub use error::{
    AppError, AppResult, CliError, ConfigError, PermissionError, PlatformError, RegistryError,
    ServiceError, UpdateError,
};
pub use platform::{
    ElevationProvider, FileSystem, PlatformServices, RegistryProvider, ResourceProvider,
    ServiceProvider, ServiceStatus,
};
