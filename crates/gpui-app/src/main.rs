mod actions;
mod app;
mod app_state;
mod components;
mod config_state;
mod route_editor_state;
mod system_status_state;
mod window_controller;
mod theme;
mod views;

use app::run;

fn main() {
    run();
}
