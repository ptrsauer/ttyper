use std::fs;
use std::process::Command;

/// Get path to the ttyper binary built by cargo.
fn ttyper_bin() -> String {
    env!("CARGO_BIN_EXE_ttyper").to_string()
}

/// Create a unique temp directory to avoid collisions with parallel test runs.
fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ttyper_{}_{}", name, std::process::id()))
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

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on empty stdin: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on empty stdin: {}",
        stderr
    );

    assert!(
        stderr.contains("empty"),
        "Expected error message about empty word list, got: {}",
        stderr
    );
}

#[test]
fn empty_file_exits_cleanly() {
    let dir = unique_temp_dir("empty_file");
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
        output.status.success(),
        "ttyper exited with non-zero status on empty file: {}",
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
fn empty_language_file_exits_cleanly() {
    let dir = unique_temp_dir("empty_langfile");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let empty_file = dir.join("empty_lang.txt");
    fs::write(&empty_file, "").unwrap();

    let output = Command::new(ttyper_bin())
        .arg("--language-file")
        .arg(&empty_file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on empty --language-file: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on empty --language-file: {}",
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
    let dir = unique_temp_dir("whitespace_file");
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

    // Whitespace-only files produce empty strings as words — ttyper treats these as
    // valid words and enters the TUI. In a headless test environment, enable_raw_mode()
    // will fail (no terminal), so we can't assert success. The key requirement is: no panic.
    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on whitespace-only file: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}
