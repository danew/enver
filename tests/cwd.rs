use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn cwd_applies_to_discovery_files_and_config() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("enver-cwd-{}-{unique}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join(".env"), "TEST_KEY=hello\n").unwrap();
    fs::write(dir.join("custom.env"), "TEST_KEY=custom\n").unwrap();
    fs::write(
        dir.join("config.toml"),
        "public_key_patterns = ['^TEST_KEY$']\n",
    )
    .unwrap();

    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_enver"))
            .arg("--cwd")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    assert_eq!(run(&["ls"])[0]["file"], ".env");
    assert_eq!(
        run(&["--config", "config.toml", "get", "TEST_KEY"])["value"],
        "hello"
    );
    assert_eq!(
        run(&[
            "--file",
            "custom.env",
            "--config",
            "config.toml",
            "get",
            "TEST_KEY"
        ])["value"],
        "custom"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_enver"))
        .arg("--cwd")
        .arg(dir.join("missing"))
        .arg("ls")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot change directory"));
    fs::remove_dir_all(dir).unwrap();
}
