#[test]
fn cli_flow_does_not_contain_sensitive_command_log_fields() {
    let main_source = include_str!("../src/main.rs");

    for forbidden in ["\"args:", "\"command:", "\"cmdLine:"] {
        assert!(
            !main_source.contains(forbidden),
            "CLI flow must not log the sensitive field {forbidden}"
        );
    }
}
