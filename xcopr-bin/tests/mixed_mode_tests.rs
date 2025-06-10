use std::process::Command;
use std::io::Write;

#[test]
fn test_mixed_numbered_and_inline() {
    // Test: -c 'cut -f1' -s '%1 has foo=%{cut -f2 | jq .foo}'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f1", 
                "-s", "%1 has foo=%{cut -f2 | jq .foo}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"alice\t{\"foo\":42,\"bar\":1}\n").expect("Failed to write to stdin");
        stdin.write_all(b"bob\t{\"foo\":99,\"bar\":2}\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines[0], "alice has foo=42");
    assert_eq!(lines[1], "bob has foo=99");
}

#[test]
fn test_mixed_multiple_inline_with_numbered() {
    // Test multiple inline commands mixed with numbered references
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "wc -w", 
                "-s", "Words: %1, Chars: %{wc -c}, Upper: %{tr a-z A-Z}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"hello world\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Words: 2, Chars: 12, Upper: HELLO WORLD");
}

#[test]
fn test_mixed_repeated_references() {
    // Test that numbered and inline references can be repeated
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f1", 
                "-s", "%1-%{cut -f2}-%1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"alpha\tbeta\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "alpha-beta-alpha");
}

#[test]
fn test_chained_coprocess_pattern() {
    // Test the %1{command} pattern (chained coprocess)
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f2", 
                "-s", "Value: %1{jq .foo}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"name\t{\"foo\":123,\"bar\":456}\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Value: 123");
}