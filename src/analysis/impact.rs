use anyhow::{Context, Result};
use crate::graph::neo4j::Neo4jGraph;
use crate::models::ImpactNode;

/// Parse a node reference like "Service:postgresql" into (type, id).
pub fn parse_node_ref(node: &str) -> (&str, &str) {
    if let Some(pos) = node.find(':') {
        (&node[..pos], &node[pos + 1..])
    } else {
        ("Service", node)
    }
}

/// Analyze downstream impact from a given node.
pub async fn analyze_impact(neo: &Neo4jGraph, node: &str) -> Result<ImpactNode> {
    let (node_type, node_id) = parse_node_ref(node);

    let downstream = neo
        .get_impact_downstream(node_type, node_id)
        .await
        .context("Failed to query impact from Neo4j")?;

    // Build a tree from the flat list of (type, id, relation, depth)
    let mut root = ImpactNode {
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        relation: "ROOT".to_string(),
        children: Vec::new(),
    };

    // Group by depth and build hierarchy
    // Simple approach: depth 1 = direct children, depth 2 = grandchildren, etc.
    let mut depth_1: Vec<ImpactNode> = Vec::new();

    for (target_type, target_id, rel_type, depth) in &downstream {
        if *depth == 1 {
            let mut child = ImpactNode::leaf(
                target_id.clone(),
                target_type.clone(),
                rel_type.clone(),
            );

            // Find depth-2 children for this node
            for (t2_type, t2_id, t2_rel, t2_depth) in &downstream {
                if *t2_depth == 2 {
                    // Check if this is a child of the current depth-1 node
                    // by looking for the connection in the data
                    child.children.push(ImpactNode::leaf(
                        t2_id.clone(),
                        t2_type.clone(),
                        t2_rel.clone(),
                    ));
                }
            }

            depth_1.push(child);
        }
    }

    // Deduplicate children
    let mut seen = std::collections::HashSet::new();
    for child in depth_1 {
        let key = format!("{}:{}", child.node_type, child.node_id);
        if seen.insert(key) {
            root.children.push(child);
        }
    }

    // Deduplicate grandchildren
    for child in &mut root.children {
        let mut seen = std::collections::HashSet::new();
        child.children.retain(|gc| {
            let key = format!("{}:{}", gc.node_type, gc.node_id);
            seen.insert(key)
        });
    }

    Ok(root)
}
