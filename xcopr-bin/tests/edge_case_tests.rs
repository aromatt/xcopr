use std::process::Command;
use std::io::Write;

#[test]
fn test_empty_input() {
    // Test behavior with no input
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "wc -l", "-s", "Lines: %1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        // Write nothing - close stdin immediately
        drop(stdin);
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With empty input, should produce no output
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_no_newline_input() {
    // Test with input that doesn't end in newline (shouldn't happen in practice but good to test)
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "Echo: %{cat}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"hello").expect("Failed to write to stdin");
        // Note: no newline at end
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should still work
    assert_eq!(stdout.trim(), "Echo: hello");
}

#[test]
fn test_template_with_no_tokens() {
    // Test template that has no % references
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "Static text output"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"anything\n").expect("Failed to write to stdin");
        stdin.write_all(b"goes here\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines[0], "Static text output");
    assert_eq!(lines[1], "Static text output");
}

#[test]
fn test_repeated_token_substitution() {
    // Test that tokens can appear multiple times in template
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "tr a-z A-Z", "-s", "%1-%1-%1"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"test\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "TEST-TEST-TEST");
}

#[test]
fn test_whitespace_handling() {
    // Test that whitespace in input and commands is handled correctly
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-s", "Trimmed: '%{echo \"  spaced  \" | xargs}'"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"  input with spaces  \n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Trimmed: 'spaced'");
}

#[test]
fn test_special_characters_in_template() {
    // Test template with special characters that might interfere with parsing
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "echo hello", "-s", "Result: [%1] (done)"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"input\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Result: [hello] (done)");
}

#[test]
fn test_complex_json_manipulation() {
    // Test complex JSON processing across multiple operations
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-s", "Sum: %{echo '{\"a\":5,\"b\":3}' | jq '.a + .b'}, Product: %{echo '{\"a\":5,\"b\":3}' | jq '.a * .b'}"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"dummy input\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Sum: 8, Product: 15");
}