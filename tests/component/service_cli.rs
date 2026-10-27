use std::process::Command;

#[test]
fn test_service_cli_subcommands_symmetric_help() {
    let exe = env!("CARGO_BIN_EXE_clawcrew");
    for sub in ["install", "status", "uninstall"] {
        let output = Command::new(exe)
            .args(["service", sub, "--help"])
            .output()
            .expect("should run service help");
        assert!(
            output.status.success(),
            "service {} --help failed: {}",
            sub,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
