use anyhow::{Context, Result};
use crate::graph::neo4j::Neo4jGraph;

use super::impact::parse_node_ref;

/// Find the shortest path between two nodes in the dependency graph.
pub async fn find_path(neo: &Neo4jGraph, from: &str, to: &str) -> Result<Option<Vec<String>>> {
    let (from_type, from_id) = parse_node_ref(from);
    let (to_type, to_id) = parse_node_ref(to);

    let path = neo
        .find_shortest_path(from_type, from_id, to_type, to_id)
        .await
        .context("Failed to query path from Neo4j")?;

    Ok(path)
}
