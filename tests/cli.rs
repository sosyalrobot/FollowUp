use std::process::Command;

#[test]
fn prints_help_without_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_followup"))
        .output()
        .expect("run followup binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FollowUp"));
    assert!(stdout.contains("Commands:"));
}
