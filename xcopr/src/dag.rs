use petgraph::graph::{Graph, NodeIndex};
use petgraph::{algo, Direction};
use std::collections::HashMap;
use crate::parser::StreamDef;

#[derive(Debug)]
pub struct Dag {
    graph: Graph<DagNode, ()>,
    pub execution_order: Vec<NodeIndex>,
}

#[derive(Debug, Clone)]
pub enum DagNode {
    Stream(StreamDef),
    Main(String),
}

#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("Cycle detected in stream dependencies")]
    CycleDetected,
    #[error("Topological sort failed")]
    TopologicalSortFailed,
}

pub fn build_dag(streams: Vec<StreamDef>, main_cmd: String) -> Result<Dag, DagError> {
    let mut graph = Graph::new();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();
    
    // Create nodes for all streams
    for stream in &streams {
        let node_idx = graph.add_node(DagNode::Stream(stream.clone()));
        node_map.insert(stream.token.clone(), node_idx);
    }
    
    // Create node for main command
    let main_node = graph.add_node(DagNode::Main(main_cmd.clone()));
    
    // Analyze dependencies and create edges
    // For each stream, check if its template contains tokens from other streams
    for stream in &streams {
        let consumer_node = node_map[&stream.token];
        
        // Check if this stream's template references other stream tokens
        for other_stream in &streams {
            if stream.id != other_stream.id && stream.template.contains(&other_stream.token) {
                let producer_node = node_map[&other_stream.token];
                graph.add_edge(producer_node, consumer_node, ());
            }
        }
    }
    
    // Check if main command references any stream tokens
    for stream in &streams {
        if main_cmd.contains(&stream.token) {
            let producer_node = node_map[&stream.token];
            graph.add_edge(producer_node, main_node, ());
        }
    }
    
    // Check for cycles
    if algo::is_cyclic_directed(&graph) {
        return Err(DagError::CycleDetected);
    }
    
    // Compute topological sort
    let execution_order = algo::toposort(&graph, None)
        .map_err(|_| DagError::TopologicalSortFailed)?;
    
    Ok(Dag {
        graph,
        execution_order,
    })
}

impl Dag {
    pub fn get_node(&self, node_idx: NodeIndex) -> Option<&DagNode> {
        self.graph.node_weight(node_idx)
    }
    
    pub fn get_dependencies(&self, node_idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(node_idx, Direction::Incoming)
            .collect()
    }
    
    pub fn get_dependents(&self, node_idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(node_idx, Direction::Outgoing)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_stream_def(id: usize, template: &str, token: &str) -> StreamDef {
        StreamDef {
            id,
            template: template.to_string(),
            token: token.to_string(),
        }
    }

    #[test]
    fn test_linear_chain() {
        // Stream 1 -> Stream 2 -> Main
        let streams = vec![
            create_stream_def(1, "echo hello", "__XCOPR_001__"),
            create_stream_def(2, "process __XCOPR_001__", "__XCOPR_002__"),
        ];
        let main_cmd = "output __XCOPR_002__".to_string();
        
        let dag = build_dag(streams, main_cmd).unwrap();
        
        // Should have 3 nodes total
        assert_eq!(dag.execution_order.len(), 3);
        
        // Verify execution order respects dependencies
        let mut stream1_pos = None;
        let mut stream2_pos = None;
        let mut main_pos = None;
        
        for (pos, &node_idx) in dag.execution_order.iter().enumerate() {
            match dag.get_node(node_idx).unwrap() {
                DagNode::Stream(stream) => {
                    if stream.id == 1 {
                        stream1_pos = Some(pos);
                    } else if stream.id == 2 {
                        stream2_pos = Some(pos);
                    }
                }
                DagNode::Main(_) => {
                    main_pos = Some(pos);
                }
            }
        }
        
        // Stream 1 should come before Stream 2, which should come before Main
        assert!(stream1_pos.unwrap() < stream2_pos.unwrap());
        assert!(stream2_pos.unwrap() < main_pos.unwrap());
    }

    #[test]
    fn test_branching_graph() {
        // Stream 1 -> Stream 2
        //          \-> Stream 3
        // Both Stream 2 and 3 -> Main
        let streams = vec![
            create_stream_def(1, "echo hello", "__XCOPR_001__"),
            create_stream_def(2, "process __XCOPR_001__", "__XCOPR_002__"),
            create_stream_def(3, "transform __XCOPR_001__", "__XCOPR_003__"),
        ];
        let main_cmd = "combine __XCOPR_002__ __XCOPR_003__".to_string();
        
        let dag = build_dag(streams, main_cmd).unwrap();
        
        // Should have 4 nodes total
        assert_eq!(dag.execution_order.len(), 4);
        
        // Find positions
        let mut stream1_pos = None;
        let mut stream2_pos = None;
        let mut stream3_pos = None;
        let mut main_pos = None;
        
        for (pos, &node_idx) in dag.execution_order.iter().enumerate() {
            match dag.get_node(node_idx).unwrap() {
                DagNode::Stream(stream) => {
                    if stream.id == 1 {
                        stream1_pos = Some(pos);
                    } else if stream.id == 2 {
                        stream2_pos = Some(pos);
                    } else if stream.id == 3 {
                        stream3_pos = Some(pos);
                    }
                }
                DagNode::Main(_) => {
                    main_pos = Some(pos);
                }
            }
        }
        
        // Stream 1 should come before both Stream 2 and 3
        assert!(stream1_pos.unwrap() < stream2_pos.unwrap());
        assert!(stream1_pos.unwrap() < stream3_pos.unwrap());
        // Both Stream 2 and 3 should come before Main
        assert!(stream2_pos.unwrap() < main_pos.unwrap());
        assert!(stream3_pos.unwrap() < main_pos.unwrap());
    }

    #[test]
    fn test_cycle_detection() {
        // Create a cycle: Stream 1 -> Stream 2 -> Stream 1
        let streams = vec![
            create_stream_def(1, "process __XCOPR_002__", "__XCOPR_001__"),
            create_stream_def(2, "process __XCOPR_001__", "__XCOPR_002__"),
        ];
        let main_cmd = "output result".to_string();
        
        let result = build_dag(streams, main_cmd);
        
        assert!(matches!(result, Err(DagError::CycleDetected)));
    }

    #[test]
    fn test_no_dependencies() {
        // Independent streams with no cross-references
        let streams = vec![
            create_stream_def(1, "echo hello", "__XCOPR_001__"),
            create_stream_def(2, "echo world", "__XCOPR_002__"),
        ];
        let main_cmd = "combine __XCOPR_001__ __XCOPR_002__".to_string();
        
        let dag = build_dag(streams, main_cmd).unwrap();
        
        // Should have 3 nodes total
        assert_eq!(dag.execution_order.len(), 3);
        
        // Both streams should come before main, but order between streams doesn't matter
        let mut main_pos = None;
        let mut stream_positions = Vec::new();
        
        for (pos, &node_idx) in dag.execution_order.iter().enumerate() {
            match dag.get_node(node_idx).unwrap() {
                DagNode::Stream(_) => {
                    stream_positions.push(pos);
                }
                DagNode::Main(_) => {
                    main_pos = Some(pos);
                }
            }
        }
        
        // All streams should come before main
        for stream_pos in stream_positions {
            assert!(stream_pos < main_pos.unwrap());
        }
    }

    #[test]
    fn test_no_streams() {
        // Just a main command with no streams
        let streams = vec![];
        let main_cmd = "echo hello".to_string();
        
        let dag = build_dag(streams, main_cmd).unwrap();
        
        // Should have 1 node (just main)
        assert_eq!(dag.execution_order.len(), 1);
        
        match dag.get_node(dag.execution_order[0]).unwrap() {
            DagNode::Main(cmd) => assert_eq!(cmd, "echo hello"),
            _ => panic!("Expected main node"),
        }
    }
}