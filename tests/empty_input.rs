use std::fs;
use std::process::Command;

/// Get path to the ttyper binary built by cargo.
fn ttyper_bin() -> String {
    env!("CARGO_BIN_EXE_ttyper").to_string()
}

#[test]
fn empty_stdin_exits_cleanly() {
    let output = Command::new(ttyper_bin())
        .arg("-")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Must not panic
    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on empty stdin: {}",
        stderr
    );

    // Should show a helpful error message
    assert!(
        stderr.contains("empty"),
        "Expected error message about empty word list, got: {}",
        stderr
    );
}

#[test]
fn empty_file_exits_cleanly() {
    let dir = std::env::temp_dir().join("ttyper_test_empty_file");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let empty_file = dir.join("empty.txt");
    fs::write(&empty_file, "").unwrap();

    let output = Command::new(ttyper_bin())
        .arg(&empty_file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on empty file: {}",
        stderr
    );

    assert!(
        stderr.contains("empty"),
        "Expected error message about empty word list, got: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn whitespace_only_file_exits_cleanly() {
    let dir = std::env::temp_dir().join("ttyper_test_whitespace_file");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("whitespace.txt");
    fs::write(&file, "\n\n\n").unwrap();

    let output = Command::new(ttyper_bin())
        .arg(&file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Whitespace-only files produce empty strings as words — ttyper may or may not
    // consider these valid. The key requirement is: no panic.
    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on whitespace-only file: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}
