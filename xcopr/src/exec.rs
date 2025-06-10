use crate::dag::{Dag, DagNode};
use crate::parser::StreamDef;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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

struct ProcessHandle {
    child: Child,
    output_receiver: mpsc::Receiver<String>,
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
    
    // Build a map from node indices to their output channels
    let mut node_outputs: HashMap<NodeIndex, mpsc::Receiver<String>> = HashMap::new();
    let mut process_handles: Vec<tokio::task::JoinHandle<Result<(), ExecError>>> = Vec::new();
    
    // Execute nodes in topological order
    for &node_idx in &dag.execution_order {
        let node = dag.get_node(node_idx).ok_or_else(|| {
            ExecError::DagExecutionFailed("Invalid node index in execution order".to_string())
        })?;
        
        match node {
            DagNode::Stream(stream_def) => {
                // Get input from dependencies
                let dependencies = dag.get_dependencies(node_idx);
                let mut substituted_template = stream_def.template.clone();
                
                // Substitute tokens with actual values from dependency outputs
                for dep_idx in dependencies {
                    if let Some(dep_node) = dag.get_node(dep_idx) {
                        if let DagNode::Stream(dep_stream) = dep_node {
                            // For now, we'll use a placeholder approach
                            // In a real implementation, we'd need to coordinate the token substitution
                            // with the actual process outputs
                            substituted_template = substituted_template.replace(
                                &dep_stream.token,
                                &format!("{{output_from_{}}}", dep_stream.id),
                            );
                        }
                    }
                }
                
                // Spawn the process
                let (output_sender, output_receiver) = mpsc::channel::<String>(100);
                node_outputs.insert(node_idx, output_receiver);
                
                let template_clone = substituted_template.clone();
                let handle = tokio::spawn(async move {
                    spawn_and_capture_process(&template_clone, output_sender).await
                });
                
                process_handles.push(handle);
            }
            DagNode::Main(main_cmd) => {
                // For the main command, substitute tokens and execute
                let mut substituted_main = main_cmd.clone();
                
                for stream in stream_defs {
                    if substituted_main.contains(&stream.token) {
                        // For now, use placeholder substitution
                        substituted_main = substituted_main.replace(
                            &stream.token,
                            &format!("{{output_from_{}}}", stream.id),
                        );
                    }
                }
                
                // Execute main command and output to stdout
                let handle = tokio::spawn(async move {
                    execute_main_command(&substituted_main).await
                });
                
                process_handles.push(handle);
            }
        }
    }
    
    // Wait for all processes to complete
    for handle in process_handles {
        handle.await.map_err(|e| {
            ExecError::DagExecutionFailed(format!("Process execution failed: {}", e))
        })??;
    }
    
    Ok(())
}

async fn spawn_and_capture_process(
    template: &str,
    output_sender: mpsc::Sender<String>,
) -> Result<(), ExecError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(template)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let stdout = child.stdout.take().ok_or_else(|| {
        ExecError::DagExecutionFailed("Failed to capture stdout".to_string())
    })?;
    
    let stderr = child.stderr.take().ok_or_else(|| {
        ExecError::DagExecutionFailed("Failed to capture stderr".to_string())
    })?;
    
    // Handle stdout
    let output_sender_clone = output_sender.clone();
    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if output_sender_clone.send(line).await.is_err() {
                break; // Receiver dropped
            }
        }
    });
    
    // Handle stderr (forward to parent stderr)
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("{}", line);
        }
    });
    
    // Wait for process to complete
    let exit_status = child.wait().await?;
    
    // Wait for output handlers to complete
    let _ = tokio::join!(stdout_handle, stderr_handle);
    
    if !exit_status.success() {
        return Err(ExecError::ProcessFailed(
            exit_status.code().unwrap_or(-1),
        ));
    }
    
    Ok(())
}

async fn execute_main_command(template: &str) -> Result<(), ExecError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(template)
        .stdout(Stdio::inherit()) // Output directly to parent stdout
        .stderr(Stdio::inherit()) // Output directly to parent stderr
        .spawn()?;
    
    let exit_status = child.wait().await?;
    
    if !exit_status.success() {
        return Err(ExecError::ProcessFailed(
            exit_status.code().unwrap_or(-1),
        ));
    }
    
    Ok(())
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
    async fn test_process_spawning() {
        let (sender, mut receiver) = mpsc::channel(10);
        
        let handle = tokio::spawn(async move {
            spawn_and_capture_process("echo 'test output'", sender).await
        });
        
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        
        // Should receive the output
        if let Some(output) = receiver.recv().await {
            assert_eq!(output, "test output");
        }
    }
    
    #[tokio::test]
    async fn test_main_command_execution() {
        // Test that main command execution works
        let result = execute_main_command("echo 'main command test' > /dev/null").await;
        assert!(result.is_ok());
    }
}