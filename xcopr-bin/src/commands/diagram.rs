use thiserror::Error;
use xcopr::dag::{build_dag, Dag, DagNode};
use xcopr::parser::StreamDef;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum DiagramError {
    #[error("DAG construction failed: {0}")]
    DagError(#[from] xcopr::dag::DagError),
    #[error("Diagram rendering failed: {0}")]
    RenderError(String),
}

pub fn run_diagram(streams: &[StreamDef], main: &str) -> Result<(), DiagramError> {
    // Build the DAG
    let dag = build_dag(streams.to_vec(), main.to_string())?;
    
    // Render the ASCII diagram
    render_ascii_dag(&dag)?;
    
    Ok(())
}

fn render_ascii_dag(dag: &Dag) -> Result<(), DiagramError> {
    println!("# xcopr Execution Graph");
    println!();
    
    // Create a mapping from node indices to display names
    let mut node_labels: HashMap<NodeIndex, String> = HashMap::new();
    let mut stream_nodes = Vec::new();
    let mut main_node = None;
    
    // Collect node information
    for &node_idx in &dag.execution_order {
        if let Some(node) = dag.get_node(node_idx) {
            match node {
                DagNode::Stream(stream) => {
                    let label = format!("Stream{}: {}", stream.id, truncate_command(&stream.template, 30));
                    node_labels.insert(node_idx, label);
                    stream_nodes.push(node_idx);
                }
                DagNode::Main(cmd) => {
                    let label = format!("Main: {}", truncate_command(cmd, 30));
                    node_labels.insert(node_idx, label);
                    main_node = Some(node_idx);
                }
            }
        }
    }
    
    // Render execution order
    println!("## Execution Order:");
    for (i, &node_idx) in dag.execution_order.iter().enumerate() {
        if let Some(label) = node_labels.get(&node_idx) {
            println!("{}. [{}]", i + 1, label);
        }
    }
    println!();
    
    // Render dependency graph
    println!("## Dependency Graph:");
    
    // For each node, show its dependencies and dependents
    for &node_idx in &dag.execution_order {
        if let Some(label) = node_labels.get(&node_idx) {
            let dependencies = dag.get_dependencies(node_idx);
            let dependents = dag.get_dependents(node_idx);
            
            if !dependencies.is_empty() || !dependents.is_empty() {
                println!("[{}]", label);
                
                // Show incoming dependencies
                if !dependencies.is_empty() {
                    for &dep_idx in &dependencies {
                        if let Some(dep_label) = node_labels.get(&dep_idx) {
                            println!("  ◀── [{}]", dep_label);
                        }
                    }
                }
                
                // Show outgoing dependencies
                if !dependents.is_empty() {
                    for &dep_idx in &dependents {
                        if let Some(dep_label) = node_labels.get(&dep_idx) {
                            println!("  ──▶ [{}]", dep_label);
                        }
                    }
                }
                println!();
            }
        }
    }
    
    // Simple linear flow visualization
    if dag.execution_order.len() > 1 {
        println!("## Flow Diagram:");
        for (i, &node_idx) in dag.execution_order.iter().enumerate() {
            if let Some(label) = node_labels.get(&node_idx) {
                // Print node
                println!("┌─{}─┐", "─".repeat(label.len()));
                println!("│ {} │", label);
                println!("└─{}─┘", "─".repeat(label.len()));
                
                // Print arrow to next node (if not last)
                if i < dag.execution_order.len() - 1 {
                    println!("    │");
                    println!("    ▼");
                }
            }
        }
    }
    
    Ok(())
}

fn truncate_command(cmd: &str, max_len: usize) -> String {
    if cmd.len() <= max_len {
        cmd.to_string()
    } else {
        format!("{}...", &cmd[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};
    
    // Helper to capture stdout for testing - simplified for now
    fn _capture_stdout<F>(_f: F) -> String 
    where 
        F: FnOnce() -> Result<(), DiagramError>,
    {
        // This is a simplified test - in a real implementation we'd use
        // a more sophisticated stdout capture mechanism
        "diagram output".to_string()
    }
    
    #[test]
    fn test_simple_diagram() {
        let streams = vec![
            StreamDef {
                id: 1,
                template: "echo hello".to_string(),
                token: "__XCOPR_001__".to_string(),
                input_source: None,
            }
        ];
        let main_cmd = "cat __XCOPR_001__";
        
        let result = run_diagram(&streams, main_cmd);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_branching_diagram() {
        let streams = vec![
            StreamDef {
                id: 1,
                template: "echo hello".to_string(),
                token: "__XCOPR_001__".to_string(),
                input_source: None,
            },
            StreamDef {
                id: 2,
                template: "process __XCOPR_001__".to_string(),
                token: "__XCOPR_002__".to_string(),
                input_source: None,
            },
            StreamDef {
                id: 3,
                template: "transform __XCOPR_001__".to_string(),
                token: "__XCOPR_003__".to_string(),
                input_source: None,
            }
        ];
        let main_cmd = "combine __XCOPR_002__ __XCOPR_003__";
        
        let result = run_diagram(&streams, main_cmd);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_truncate_command() {
        assert_eq!(truncate_command("short", 10), "short");
        assert_eq!(truncate_command("this is a very long command", 10), "this is...");
        assert_eq!(truncate_command("exactly10!", 10), "exactly10!");
    }
    
    #[test]
    fn test_cycle_detection_in_diagram() {
        let streams = vec![
            StreamDef {
                id: 1,
                template: "process __XCOPR_002__".to_string(),
                token: "__XCOPR_001__".to_string(),
                input_source: None,
            },
            StreamDef {
                id: 2,
                template: "process __XCOPR_001__".to_string(),
                token: "__XCOPR_002__".to_string(),
                input_source: None,
            }
        ];
        let main_cmd = "output result";
        
        let result = run_diagram(&streams, main_cmd);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DiagramError::DagError(_)));
    }
}