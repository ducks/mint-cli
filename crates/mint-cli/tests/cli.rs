use std::process::{Command, Stdio};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mint-cli"))
}
#[test]
fn help_exposes_real_commands_but_no_purchase_or_credential_flags() {
    let result = cli().arg("--help").output().unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    for name in ["login", "logout", "status", "usage", "plans"] {
        assert!(text.contains(name));
    }
    assert!(!text.contains("--password"));
    let result = cli().args(["plans", "renew"]).output().unwrap();
    assert!(!result.status.success());
}
#[test]
fn noninteractive_login_never_contacts_mint_or_accepts_password_input() {
    // Temporary XDG state avoids changing the user's normal state directory.
    let directory = std::env::temp_dir().join(format!("mint-cli-test-{}", std::process::id()));
    let result = cli()
        .args(["login", "--json"])
        .env("XDG_STATE_HOME", &directory)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["schema_version"], 1);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("interactive terminal")
    );
    std::fs::remove_dir_all(directory).unwrap();
}
