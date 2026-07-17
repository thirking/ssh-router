#![allow(dead_code)]

use gpui::{AppContext, Context, Entity, Window};
use gpui_component::input::InputState;
use ssh_router_config::Route;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteEditorState {
    pub route: RouteDraft,
    pub editing_id: Option<u64>,
    pub errors: Vec<String>,
}

pub struct RouteEditorInputs {
    pub port: Entity<InputState>,
    pub name: Entity<InputState>,
    pub shell: Entity<InputState>,
    pub interactive_template: Entity<InputState>,
    pub command_template: Entity<InputState>,
    pub tmp_file_ext: Entity<InputState>,
    pub sftp_command: Entity<InputState>,
}

impl RouteEditorInputs {
    pub fn new(window: &mut Window, cx: &mut Context<crate::app_state::AppState>) -> Self {
        let mut new_input =
            |cx: &mut Context<crate::app_state::AppState>| cx.new(|cx| InputState::new(window, cx));
        Self {
            port: new_input(cx),
            name: new_input(cx),
            shell: new_input(cx),
            interactive_template: new_input(cx),
            command_template: new_input(cx),
            tmp_file_ext: cx.new(|cx| InputState::new(window, cx).default_value(".sh")),
            sftp_command: cx.new(|cx| InputState::new(window, cx)),
        }
    }

    pub fn set_sftp_command(
        &self,
        value: String,
        window: &mut Window,
        cx: &mut Context<crate::app_state::AppState>,
    ) {
        self.sftp_command
            .update(cx, |input, cx| input.set_value(value, window, cx));
    }

    pub fn set_route(
        &self,
        route: &RouteDraft,
        window: &mut Window,
        cx: &mut Context<crate::app_state::AppState>,
    ) {
        let values = [
            (&self.port, route.port.clone()),
            (&self.name, route.name.clone()),
            (&self.shell, route.shell.clone()),
            (
                &self.interactive_template,
                route.interactive_template.clone(),
            ),
            (&self.command_template, route.command_template.clone()),
            (&self.tmp_file_ext, route.tmp_file_ext.clone()),
        ];
        for (input, value) in values {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_draft_converts_to_route() {
        let mut editor = RouteEditorState {
            route: RouteDraft {
                port: "2200".to_string(),
                name: "测试路由".to_string(),
                shell: "pwsh.exe".to_string(),
                interactive_template: "{shell} -l".to_string(),
                command_template: "{shell} -File {tmpfile}".to_string(),
                tmp_file_ext: ".ps1".to_string(),
                default: false,
            },
            editing_id: None,
            errors: Vec::new(),
        };

        let route = editor.validate().expect("有效草稿应通过校验");

        assert_eq!(route.port, 2200);
        assert_eq!(route.name, "测试路由");
    }

    #[test]
    fn cancelling_editor_does_not_change_source_route() {
        let source = ssh_router_config::Config::default_config().routes[0].clone();
        let mut editor = RouteEditorState::new(Some((7, source.clone())));
        editor.route.name = "未保存名称".to_string();

        editor.cancel();

        assert_eq!(source.name, "PowerShell");
        assert_eq!(editor.editing_id, Some(7));
    }
}
