use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use ssh_router_config::Config;

fn exchange(path: &std::path::Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ssh-router-backend"))
        .arg("--config")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("后端应能启动");
    let mut input = child.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let replies = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert!(child.wait().unwrap().success());
    replies
}

#[test]
fn protocol_persists_unicode_config_and_rejects_duplicate_ports() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut config = Config::default_config();
    config.routes[1].name = "中文路由\n带换行和引号\"".into();
    let mut invalid = config.clone();
    invalid.routes[1].port = invalid.routes[0].port;
    let replies = exchange(
        &path,
        &[
            json!({"id": 1, "method": "hello"}),
            json!({"id": 2, "method": "config.load"}),
            json!({"id": 3, "method": "config.save", "params": config}),
            json!({"id": 4, "method": "config.save", "params": invalid}),
            json!({"id": 5, "method": "config.load"}),
        ],
    );
    assert_eq!(replies[0]["result"]["protocol"], 1);
    assert_eq!(replies[1]["error"]["code"], "not_found");
    assert_eq!(replies[2]["result"], json!(config));
    assert_eq!(replies[3]["error"]["code"], "validation");
    assert_eq!(replies[4]["result"], json!(config));
    assert_eq!(
        serde_json::from_slice::<Config>(&std::fs::read(path).unwrap()).unwrap(),
        config
    );
}

#[test]
fn create_does_not_overwrite_existing_or_corrupt_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let replies = exchange(
        &path,
        &[
            json!({"id": 1, "method": "config.create"}),
            json!({"id": 2, "method": "config.create"}),
        ],
    );
    assert!(replies[0].get("result").is_some());
    assert_eq!(replies[1]["error"]["code"], "validation");
    std::fs::write(&path, "{broken").unwrap();
    let replies = exchange(&path, &[json!({"id": 3, "method": "config.create"})]);
    assert_eq!(replies[0]["error"]["code"], "invalid_format");
    assert_eq!(std::fs::read_to_string(path).unwrap(), "{broken");
}

#[test]
fn malformed_and_unknown_requests_do_not_stop_later_requests() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let replies = exchange(
        &path,
        &[
            json!({"method": "config.load"}),
            json!({"id": 2, "method": "shell.execute", "params": "never run"}),
            json!({"id": 3, "method": "config.defaults"}),
        ],
    );
    assert_eq!(replies[0]["error"]["code"], "protocol");
    assert_eq!(replies[1]["error"]["code"], "validation");
    assert_eq!(replies[2]["result"], json!(Config::default_config()));
    assert!(!path.exists());
}

#[test]
fn legacy_configuration_is_normalized_by_shared_core() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let mut legacy = json!(Config::default_config());
    for route in legacy["routes"].as_array_mut().unwrap() {
        route.as_object_mut().unwrap().remove("default");
    }
    std::fs::write(&path, legacy.to_string()).unwrap();
    let replies = exchange(&path, &[json!({"id": 1, "method": "config.load"})]);
    assert_eq!(replies[0]["result"]["routes"][0]["default"], true);
}
