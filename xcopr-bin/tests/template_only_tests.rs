use std::process::Command;
use std::io::Write;

#[test]
fn test_template_only_simple() {
    // Test: echo 'hello world' | xcopr map -s '%{wc -w}: %{tr a-z A-Z}'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "%{wc -w}: %{tr a-z A-Z}"])
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
    assert_eq!(stdout.trim(), "2: HELLO WORLD");
}

#[test]
fn test_template_only_json_processing() {
    // Test the JSON processing example from the original request
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "%{cut -f1}: %{cut -f2 | jq \".foo == .bar\"}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"alice\t{\"foo\":0,\"bar\":1}\n").expect("Failed to write to stdin");
        stdin.write_all(b"billy\t{\"foo\":1,\"bar\":1}\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines[0], "alice: false");
    assert_eq!(lines[1], "billy: true");
}

#[test]
fn test_template_only_multiple_lines() {
    // Test with multiple input lines
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "Line %{wc -c} chars: %{cat}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"foo\n").expect("Failed to write to stdin");
        stdin.write_all(b"hello\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines[0], "Line 4 chars: foo");
    assert_eq!(lines[1], "Line 6 chars: hello");
}