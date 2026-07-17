pub mod cli_service;
pub mod config_service;
pub mod elevation_service;
pub mod error;
pub mod platform;
pub mod status_service;
pub mod signature;
pub mod update_manifest;
pub mod update_service;

pub use cli_service::files_match;
pub use config_service::{ConfigRepository, ConfigService, CONFIG_PATH};
pub use elevation_service::ElevationService;
pub use error::{
    AppError, AppResult, CliError, ConfigError, PermissionError, PlatformError, RegistryError,
    ServiceError, UpdateError,
};
pub use platform::{
    ElevationProvider, FileSystem, PlatformServices, RegistryProvider, ResourceProvider,
    ServiceProvider, ServiceStatus,
};
pub use status_service::{DesktopSystemService, OperationMessage, SystemService, SystemStatus};
pub use update_manifest::{PlatformUpdate, UpdateCandidate, UpdateManifest};
pub use update_service::UpdateService;
