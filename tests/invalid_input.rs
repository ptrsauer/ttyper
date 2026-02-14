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
fn nonexistent_file_exits_cleanly() {
    let output = Command::new(ttyper_bin())
        .arg("/tmp/ttyper_nonexistent_file_xyz_42.txt")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on nonexistent file: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on nonexistent file: {}",
        stderr
    );

    // Should mention the file path in the error
    assert!(
        stderr.contains("Cannot open"),
        "Expected error about file not found, got: {}",
        stderr
    );
}

#[test]
fn invalid_language_exits_cleanly() {
    let output = Command::new(ttyper_bin())
        .arg("-l")
        .arg("nonexistent_language_xyz_42")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on invalid language: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on invalid language: {}",
        stderr
    );

    assert!(
        stderr.contains("not found"),
        "Expected error about language not found, got: {}",
        stderr
    );

    // Should suggest --list-languages
    assert!(
        stderr.contains("--list-languages"),
        "Expected suggestion to use --list-languages, got: {}",
        stderr
    );
}

#[test]
fn nonexistent_language_file_exits_cleanly() {
    let output = Command::new(ttyper_bin())
        .arg("--language-file")
        .arg("/tmp/ttyper_nonexistent_lang_xyz_42.txt")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on nonexistent language file: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on nonexistent language file: {}",
        stderr
    );

    assert!(
        stderr.contains("Cannot read language file"),
        "Expected error about language file not found, got: {}",
        stderr
    );
}

#[test]
fn binary_language_file_exits_cleanly() {
    let dir = unique_temp_dir("binary_langfile");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("binary.bin");
    // Write invalid UTF-8 bytes
    fs::write(&file, &[0xFF, 0xFE, 0x80, 0x81, 0x00, 0xC0, 0xC1]).unwrap();

    let output = Command::new(ttyper_bin())
        .arg("--language-file")
        .arg(&file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on binary language file: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on binary language file: {}",
        stderr
    );

    assert!(
        stderr.contains("UTF-8"),
        "Expected error about UTF-8 encoding, got: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn help_as_path_exits_cleanly() {
    // Common user mistake: `ttyper help` instead of `ttyper --help`
    let output = Command::new(ttyper_bin())
        .arg("help")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute ttyper");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("panicked"),
        "ttyper panicked on 'help' as path: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "ttyper exited with non-zero status on 'help' as path: {}",
        stderr
    );

    // Should show an error about not being able to open the file
    assert!(
        stderr.contains("Cannot open"),
        "Expected file-not-found error for 'help', got: {}",
        stderr
    );
}
