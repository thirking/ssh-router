mod log;
mod wsl;
mod temp;

fn main() {
    temp::clean_stale_temp_files(std::process::id());
    log::log("========");
    log::log("ssh-router-cli starting");
    log::log("========");
}
