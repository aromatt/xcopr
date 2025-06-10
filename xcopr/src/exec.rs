use crate::dag::{Dag, DagNode};
use crate::parser::StreamDef;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("Failed to spawn process: {0}")]
    SpawnError(#[from] std::io::Error),
    #[error("Process exited with non-zero status: {0}")]
    ProcessFailed(i32),
    #[error("Token substitution failed for token: {0}")]
    TokenSubstitutionFailed(String),
    #[error("DAG execution failed: {0}")]
    DagExecutionFailed(String),
}

pub async fn execute_dag(
    dag: &Dag,
    stream_defs: &[StreamDef],
    main_template: &str,
) -> Result<(), ExecError> {
    // Build a map from tokens to their corresponding stream definitions
    let mut token_to_stream: HashMap<String, &StreamDef> = HashMap::new();
    for stream in stream_defs {
        token_to_stream.insert(stream.token.clone(), stream);
    }
    
    // For a simple implementation, let's handle the basic case:
    // 1. Read from stdin
    // 2. Process through coprocesses 
    // 3. Substitute tokens in main command
    // 4. Execute main command with substituted values
    
    // Read stdin line by line
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    
    while let Ok(Some(input_line)) = lines.next_line().await {
        // For each input line, process it through all coprocesses
        let mut token_values: HashMap<String, String> = HashMap::new();
        
        // Process each stream in execution order
        for &node_idx in &dag.execution_order {
            if let Some(node) = dag.get_node(node_idx) {
                match node {
                    DagNode::Stream(stream_def) => {
                        // Process this line through the coprocess
                        let output = process_line_through_command(&input_line, &stream_def.template).await?;
                        token_values.insert(stream_def.token.clone(), output);
                    }
                    DagNode::Main(_) => {
                        // Handle main command after all streams are processed
                        break;
                    }
                }
            }
        }
        
        // Now substitute tokens in main command and execute
        let substituted_main = substitute_tokens(main_template, &token_values);
        execute_main_command_with_input(&substituted_main, &input_line).await?;
    }
    
    Ok(())
}

async fn process_line_through_command(input: &str, command: &str) -> Result<String, ExecError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // Write input to the process
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.shutdown().await?;
    }
    
    // Read output
    let output = child.wait_with_output().await?;
    
    if !output.status.success() {
        return Err(ExecError::ProcessFailed(
            output.status.code().unwrap_or(-1),
        ));
    }
    
    // Return stdout as string, trimmed
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn execute_main_command_with_input(command: &str, input: &str) -> Result<(), ExecError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    // Write input to the process
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.shutdown().await?;
    }
    
    let exit_status = child.wait().await?;
    
    if !exit_status.success() {
        return Err(ExecError::ProcessFailed(
            exit_status.code().unwrap_or(-1),
        ));
    }
    
    Ok(())
}

pub async fn execute_map_simple(
    coprocess_cmd: &str,
    tokenized_template: &str,
    stream_defs: &[StreamDef],
) -> Result<(), ExecError> {
    // Read stdin line by line
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    
    while let Ok(Some(input_line)) = lines.next_line().await {
        // Process the line through the coprocess
        let coprocess_output = process_line_through_command(&input_line, coprocess_cmd).await?;
        
        // Build token substitution map
        let mut token_values: HashMap<String, String> = HashMap::new();
        
        // Map tokens to coprocess output
        for stream_def in stream_defs {
            if stream_def.template == "stream_1" {
                // %1 maps to the first coprocess output
                token_values.insert(stream_def.token.clone(), coprocess_output.clone());
            } else if stream_def.template.starts_with("stream_") {
                // For now, other numbered streams also map to the first coprocess
                // TODO: Implement multiple coprocesses
                token_values.insert(stream_def.token.clone(), coprocess_output.clone());
            } else {
                // This is an inline command like "jq .foo" that should process the coprocess output
                let chained_output = process_line_through_command(&coprocess_output, &stream_def.template).await?;
                token_values.insert(stream_def.token.clone(), chained_output);
            }
        }
        
        // Substitute tokens in the tokenized template
        let final_output = substitute_tokens(tokenized_template, &token_values);
        
        // Print the result to stdout
        println!("{}", final_output);
    }
    
    Ok(())
}

fn substitute_tokens(template: &str, values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (token, value) in values {
        result = result.replace(token, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::build_dag;
    use crate::parser::parse_tokens;

    #[tokio::test]
    async fn test_simple_filter_execution() {
        // This is a simplified test - in reality we'd need more sophisticated
        // token substitution and process coordination
        let template = "echo 'foo\\nbar\\nfoo' | grep foo";
        let streams = parse_tokens(template);
        
        // For now, just test that we can build a DAG and the function doesn't panic
        let dag = build_dag(streams.clone(), template.to_string());
        assert!(dag.is_ok());
        
        // Note: Full integration testing would require setting up actual process
        // coordination, which is complex for this basic implementation
    }
    
    #[tokio::test]
    async fn test_process_line_through_command() {
        let result = process_line_through_command("hello", "tr a-z A-Z").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "HELLO");
    }
    
    #[tokio::test]
    async fn test_main_command_execution() {
        // Test that main command execution works
        let result = execute_main_command_with_input("cat > /dev/null", "test input").await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_substitute_tokens() {
        let mut values = HashMap::new();
        values.insert("__XCOPR_001__".to_string(), "FOO".to_string());
        values.insert("__XCOPR_002__".to_string(), "BAR".to_string());
        
        // Test single token substitution
        let result = substitute_tokens("echo __XCOPR_001__", &values);
        assert_eq!(result, "echo FOO");
        
        // Test multiple tokens
        let result = substitute_tokens("echo __XCOPR_001__ __XCOPR_002__", &values);
        assert_eq!(result, "echo FOO BAR");
        
        // Test repeated tokens
        let result = substitute_tokens("echo __XCOPR_001__ and __XCOPR_001__ again", &values);
        assert_eq!(result, "echo FOO and FOO again");
        
        // Test no tokens
        let result = substitute_tokens("echo hello", &values);
        assert_eq!(result, "echo hello");
        
        // Test missing token (should remain unchanged)
        let result = substitute_tokens("echo __XCOPR_999__", &values);
        assert_eq!(result, "echo __XCOPR_999__");
    }
}