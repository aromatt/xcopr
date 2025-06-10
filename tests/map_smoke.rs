use std::process::Command;
use std::io::Write;

#[test]
fn test_map_mode_smoke() {
    // Test basic map functionality: echo 'foo' | xcopr map -c 'tr a-z A-Z' -s '%1'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "tr a-z A-Z", "-s", "%1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    // Write input to the process
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"foo\n").expect("Failed to write to stdin");
    }

    // Wait for process to complete and capture output
    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    // Check that token substitution works correctly
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "FOO");
}