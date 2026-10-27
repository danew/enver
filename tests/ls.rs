use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn discovers_dotenv_files_without_loading_default_env() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("enver-ls-{}-{unique}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join(".env.staging"), "not valid dotenv = secret").unwrap();
    fs::write(dir.join(".env.local"), "SECRET=hidden").unwrap();
    fs::write(dir.join(".env.root"), "").unwrap();
    fs::write(dir.join(".env.local.visibility.json"), "{}").unwrap();
    fs::create_dir(dir.join(".env.directory")).unwrap();
    fs::write(dir.join("other.env"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_enver"))
        .current_dir(&dir)
        .args(["--config", "missing.toml", "ls"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entries: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        entries,
        serde_json::json!([
            {"file": ".env.local", "alias": "local", "readable": true},
            {"file": ".env.root", "alias": null, "readable": true},
            {"file": ".env.staging", "alias": "staging", "readable": true}
        ])
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("hidden"));

    fs::write(dir.join(".env"), "SECRET=value").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_enver"))
        .current_dir(&dir)
        .arg("ls")
        .output()
        .unwrap();
    let entries: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        entries[0],
        serde_json::json!({"file": ".env", "alias": "root", "readable": true})
    );
    fs::remove_dir_all(dir).unwrap();
}
