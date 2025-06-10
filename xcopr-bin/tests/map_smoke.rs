use std::process::Command;
use std::io::Write;

#[test]
fn test_simple_coprocess_basic() {
    // Test basic map functionality: echo 'foo' | xcopr map -c 'tr a-z A-Z' -s '%1'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "tr a-z A-Z", "-s", "%1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"foo\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "FOO");
}

#[test]
fn test_simple_coprocess_with_template() {
    // Test: echo 'hello world' | xcopr map -c 'wc -w' -s 'Word count: %1'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "wc -w", "-s", "Word count: %1"])
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
    assert_eq!(stdout.trim(), "Word count: 2");
}

#[test]
fn test_simple_coprocess_multiline() {
    // Test with multiple lines of input
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "tr a-z A-Z", "-s", "Upper: %1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"foo\n").expect("Failed to write to stdin");
        stdin.write_all(b"bar\n").expect("Failed to write to stdin");
        stdin.write_all(b"baz\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines[0], "Upper: FOO");
    assert_eq!(lines[1], "Upper: BAR");
    assert_eq!(lines[2], "Upper: BAZ");
}

#[test]
fn test_simple_coprocess_pipeline() {
    // Test with a complex pipeline in the coprocess
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f2 | jq .foo", 
                "-s", "Foo value: %1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"name\t{\"foo\":42,\"bar\":99}\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Foo value: 42");
}