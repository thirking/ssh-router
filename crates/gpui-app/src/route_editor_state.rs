#![allow(dead_code)]

use ssh_router_config::Route;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteEditorState {
    pub route: RouteDraft,
    pub editing_id: Option<u64>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteDraft {
    pub port: String,
    pub name: String,
    pub shell: String,
    pub interactive_template: String,
    pub command_template: String,
    pub tmp_file_ext: String,
    pub default: bool,
}

impl Default for RouteDraft {
    fn default() -> Self {
        Self {
            port: String::new(),
            name: String::new(),
            shell: String::new(),
            interactive_template: String::new(),
            command_template: String::new(),
            tmp_file_ext: ".sh".to_string(),
            default: false,
        }
    }
}

impl RouteEditorState {
    pub fn new(route: Option<(u64, Route)>) -> Self {
        match route {
            Some((editing_id, route)) => Self {
                route: RouteDraft::from(route),
                editing_id: Some(editing_id),
                errors: Vec::new(),
            },
            None => Self::default(),
        }
    }

    pub fn cancel(&mut self) {
        self.errors.clear();
    }

    pub fn validate(&mut self) -> Result<Route, Vec<String>> {
        self.errors.clear();
        let port = match self.route.port.trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.errors
                    .push("端口必须是 1 到 65535 之间的数字".to_string());
                0
            }
        };
        for (value, label) in [
            (&self.route.name, "名称"),
            (&self.route.shell, "Shell 路径"),
            (&self.route.interactive_template, "交互式模板"),
            (&self.route.command_template, "命令模板"),
            (&self.route.tmp_file_ext, "临时文件扩展名"),
        ] {
            if value.trim().is_empty() {
                self.errors.push(format!("{label}不能为空"));
            }
        }
        if self.errors.is_empty() {
            Ok(self.route.clone().into_route(port))
        } else {
            Err(self.errors.clone())
        }
    }
}

impl From<Route> for RouteDraft {
    fn from(route: Route) -> Self {
        Self {
            port: route.port.to_string(),
            name: route.name,
            shell: route.shell,
            interactive_template: route.interactive_template,
            command_template: route.command_template,
            tmp_file_ext: route.tmp_file_ext,
            default: route.default,
        }
    }
}

impl RouteDraft {
    fn into_route(self, port: u16) -> Route {
        Route {
            port,
            name: self.name,
            shell: self.shell,
            interactive_template: self.interactive_template,
            command_template: self.command_template,
            tmp_file_ext: self.tmp_file_ext,
            default: self.default,
        }
    }
}
