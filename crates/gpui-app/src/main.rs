mod actions;
mod app;
mod app_state;
mod components;
mod config_state;
mod route_editor_state;
mod system_status_state;
mod theme;
mod update_state;
mod views;
mod window_controller;

use app::run;

fn main() {
    run();
}
