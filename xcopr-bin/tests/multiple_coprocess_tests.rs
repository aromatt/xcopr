use std::process::Command;
use std::io::Write;

#[test]
fn test_multiple_coprocesses_simple() {
    // Test: echo 'hello world' | xcopr map -c 'wc -w' -c 'tr a-z A-Z' -s '%1 words: %2'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", "-c", "wc -w", "-c", "tr a-z A-Z", "-s", "%1 words: %2"])
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
    assert_eq!(stdout.trim(), "2 words: HELLO WORLD");
}

#[test]
fn test_multiple_coprocesses_json_example() {
    // Test the original example: -c 'cut -f1' -c 'cut -f2 | jq ".foo == .bar"' -s '%1: %2'
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f1", 
                "-c", "cut -f2 | jq \".foo == .bar\"", 
                "-s", "%1: %2"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"alice\t{\"foo\":0,\"bar\":1}\n").expect("Failed to write to stdin");
        stdin.write_all(b"billy\t{\"foo\":1,\"bar\":1}\n").expect("Failed to write to stdin");
        stdin.write_all(b"charlie\t{\"bar\":0,\"foo\":1}\n").expect("Failed to write to stdin");
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
    assert_eq!(lines[2], "charlie: false");
}

#[test]
fn test_multiple_coprocesses_three_streams() {
    // Test with three coprocesses
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f1", 
                "-c", "cut -f2", 
                "-c", "wc -c",
                "-s", "Name: %1, Data: %2, Length: %3"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xcopr");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(b"foo\tbar\n").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Process failed with exit code: {:?}", output.status.code());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "Name: foo, Data: bar, Length: 8");
}

#[test]
fn test_multiple_coprocesses_reverse_order() {
    // Test that %2 comes before %1 in template (order independence)
    let mut child = Command::new("cargo")
        .args(&["run", "--", "map", 
                "-c", "cut -f1", 
                "-c", "cut -f2", 
                "-s", "Second: %2, First: %1"])
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
    assert_eq!(stdout.trim(), "Second: beta, First: alpha");
}