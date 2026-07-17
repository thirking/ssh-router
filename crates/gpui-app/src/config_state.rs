#![allow(dead_code)]

use std::collections::HashMap;

use ssh_router_app_core::{AppError, ConfigError, ConfigRepository};
use ssh_router_config::{validate_config, Config, ConfigValidationError, Route};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiRoute {
    pub id: u64,
    pub route: Route,
}

impl UiRoute {
    fn from_route(id: u64, route: Route) -> Self {
        Self { id, route }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigLoadState {
    Loading,
    NotFound,
    Corrupt(String),
    Loaded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigStateError {
    Validation(String),
    Save(String),
}

pub struct ConfigState {
    pub load_state: ConfigLoadState,
    pub saved: Option<Config>,
    pub routes: Vec<UiRoute>,
    pub sftp_command: String,
    pub is_dirty: bool,
    pub error: Option<ConfigStateError>,
    next_id: u64,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            load_state: ConfigLoadState::Loading,
            saved: None,
            routes: Vec::new(),
            sftp_command: String::new(),
            is_dirty: false,
            error: None,
            next_id: 1,
        }
    }
}

impl ConfigState {
    pub fn from_loaded(config: Config) -> Self {
        let mut state = Self::default();
        state.apply_loaded(config);
        state
    }

    pub fn apply_loaded(&mut self, config: Config) {
        self.next_id = 1;
        self.routes = config
            .routes
            .iter()
            .cloned()
            .map(|route| {
                let id = self.allocate_id();
                UiRoute::from_route(id, route)
            })
            .collect();
        self.sftp_command = config.sftp_command.clone();
        self.saved = Some(config);
        self.load_state = ConfigLoadState::Loaded;
        self.is_dirty = false;
        self.error = None;
    }

    pub fn load_from_repository(&mut self, repository: &dyn ConfigRepository) {
        self.apply_load_result(repository.load());
    }

    pub fn apply_load_result(&mut self, result: ssh_router_app_core::AppResult<Config>) {
        match result {
            Ok(config) => self.apply_loaded(config),
            Err(AppError::Config(ConfigError::NotFound { .. })) => self.mark_not_found(),
            Err(AppError::Config(ConfigError::InvalidFormat { message })) => {
                self.mark_corrupt(message)
            }
            Err(error) => {
                self.error = Some(ConfigStateError::Save(error.to_string()));
            }
        }
    }

    pub fn mark_not_found(&mut self) {
        self.load_state = ConfigLoadState::NotFound;
        self.saved = None;
        self.routes.clear();
        self.sftp_command.clear();
        self.is_dirty = false;
    }

    pub fn mark_corrupt(&mut self, message: impl Into<String>) {
        self.load_state = ConfigLoadState::Corrupt(message.into());
        self.saved = None;
        self.routes.clear();
        self.sftp_command.clear();
        self.is_dirty = false;
    }

    pub fn create_default(&mut self) {
        self.apply_loaded(Config::default_config());
        self.is_dirty = true;
    }

    pub fn to_config(&self) -> Config {
        let mut config = Config::default_config();
        config.routes = self.routes.iter().map(|item| item.route.clone()).collect();
        config.sftp_command = self.sftp_command.clone();
        config
    }

    pub fn update_route(&mut self, id: u64, route: Route) -> bool {
        let Some(item) = self.routes.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        item.route = route;
        self.normalize_default(id);
        self.is_dirty = true;
        true
    }

    pub fn add_route(&mut self, route: Route) -> u64 {
        let id = self.allocate_id();
        self.routes.push(UiRoute::from_route(id, route));
        self.normalize_default(id);
        self.is_dirty = true;
        id
    }

    pub fn delete_route(&mut self, id: u64) -> bool {
        let before = self.routes.len();
        self.routes.retain(|item| item.id != id);
        let changed = before != self.routes.len();
        if changed {
            self.is_dirty = true;
        }
        changed
    }

    pub fn set_default(&mut self, id: u64) -> bool {
        if !self.routes.iter().any(|item| item.id == id) {
            return false;
        }
        for item in &mut self.routes {
            item.route.default = item.id == id;
        }
        self.is_dirty = true;
        true
    }

    pub fn validate(&mut self) -> Result<(), ConfigStateError> {
        validate_config(&self.to_config()).map_err(|error| {
            let error = ConfigStateError::Validation(error.to_string());
            self.error = Some(error.clone());
            error
        })?;
        self.error = None;
        Ok(())
    }

    pub fn apply_save_result(
        &mut self,
        result: Result<(), impl Into<String>>,
    ) -> Result<(), ConfigStateError> {
        match result {
            Ok(()) => {
                let config = self.to_config();
                self.saved = Some(config);
                self.is_dirty = false;
                self.error = None;
                Ok(())
            }
            Err(message) => {
                let error = ConfigStateError::Save(message.into());
                self.error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn field_errors(&self) -> HashMap<&'static str, String> {
        let mut errors = HashMap::new();
        if let Err(ConfigValidationError::EmptySftpCommand) = validate_config(&self.to_config()) {
            errors.insert("sftp_command", "SFTP 命令不能为空".to_string());
        }
        errors
    }

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn normalize_default(&mut self, preferred_id: u64) {
        if self.routes.iter().filter(|item| item.route.default).count() > 1 {
            for item in &mut self.routes {
                item.route.default = item.id == preferred_id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_route_is_exactly_one_after_edit() {
        let mut state = ConfigState::from_loaded(Config::default_config());
        let id = state.routes[1].id;
        state.set_default(id);
        assert_eq!(
            state
                .routes
                .iter()
                .filter(|item| item.route.default)
                .count(),
            1
        );
        assert_eq!(state.routes[1].route.port, 2222);
    }

    #[test]
    fn duplicate_port_is_reported_by_shared_validator() {
        let mut state = ConfigState::from_loaded(Config::default_config());
        let route = state.routes[1].route.clone();
        state.update_route(state.routes[0].id, route);
        assert!(matches!(
            state.validate(),
            Err(ConfigStateError::Validation(_))
        ));
    }

    #[test]
    fn delete_marks_state_dirty_and_cancel_edit_is_non_mutating() {
        let mut state = ConfigState::from_loaded(Config::default_config());
        let original = state.routes.clone();
        assert!(!state.is_dirty);
        assert_eq!(original, state.routes);
        state.delete_route(state.routes[2].id);
        assert!(state.is_dirty);
    }

    #[test]
    fn failed_save_keeps_draft_dirty() {
        let mut state = ConfigState::from_loaded(Config::default_config());
        state.sftp_command = "changed".to_string();
        state.is_dirty = true;
        assert!(state.apply_save_result(Err("权限不足")).is_err());
        assert!(state.is_dirty);
        assert_eq!(state.sftp_command, "changed");
    }
}
