use anyhow::{Context, Result};
use neo4rs::{Graph, query};
use tracing::info;

use crate::models::*;

/// Neo4j graph layer for rootmap.
pub struct Neo4jGraph {
    graph: Graph,
}

impl Neo4jGraph {
    /// Connect to Neo4j via Bolt protocol.
    pub async fn connect(url: &str, user: &str, pass: &str) -> Result<Self> {
        let graph = Graph::new(url, user, pass)
            .await
            .context("Failed to connect to Neo4j. Is it running?")?;
        info!("Connected to Neo4j");
        Ok(Neo4jGraph { graph })
    }

    /// Create uniqueness constraints and indexes.
    pub async fn ensure_constraints(&self) -> Result<()> {
        let constraints = vec![
            "CREATE CONSTRAINT IF NOT EXISTS FOR (h:Host) REQUIRE h.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (p:Process) REQUIRE p.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (s:Service) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (c:Container) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (pt:Port) REQUIRE pt.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (i:Incident) REQUIRE i.id IS UNIQUE",
        ];

        for constraint in constraints {
            self.graph.run(query(constraint)).await
                .context("Failed to create Neo4j constraint")?;
        }

        info!("Neo4j constraints ensured");
        Ok(())
    }

    /// Sync all data from SQLite to Neo4j.
    pub async fn sync_all(&self, data: &SyncData) -> Result<SyncStats> {
        let mut stats = SyncStats::default();

        // Sync hosts
        for (id, hostname, os, kernel) in &data.hosts {
            self.graph
                .run(query(
                    "MERGE (h:Host {id: $id})
                     SET h.hostname = $hostname, h.os = $os, h.kernel = $kernel",
                )
                .param("id", id.to_string())
                .param("hostname", hostname.clone())
                .param("os", os.clone())
                .param("kernel", kernel.clone()))
                .await?;
            stats.nodes += 1;
        }

        // Sync important processes
        for (host, name, pid, status) in &data.processes {
            let proc_id = format!("{}:{}", name, pid);
            self.graph
                .run(query(
                    "MERGE (p:Process {id: $id})
                     SET p.name = $name, p.pid = $pid, p.status = $status",
                )
                .param("id", proc_id.clone())
                .param("name", name.clone())
                .param("pid", *pid as i64)
                .param("status", status.clone()))
                .await?;
            stats.nodes += 1;

            // Link process to host
            self.graph
                .run(query(
                    "MATCH (h:Host {hostname: $host}), (p:Process {id: $proc_id})
                     MERGE (h)-[:RUNS_PROCESS]->(p)",
                )
                .param("host", host.clone())
                .param("proc_id", proc_id))
                .await?;
            stats.relations += 1;
        }

        // Sync services
        for (host, name, active_state, sub_state) in &data.services {
            self.graph
                .run(query(
                    "MERGE (s:Service {id: $id})
                     SET s.name = $name, s.active_state = $active_state, s.sub_state = $sub_state",
                )
                .param("id", name.clone())
                .param("name", name.clone())
                .param("active_state", active_state.clone())
                .param("sub_state", sub_state.clone()))
                .await?;
            stats.nodes += 1;

            // Link service to host
            self.graph
                .run(query(
                    "MATCH (h:Host {hostname: $host}), (s:Service {id: $svc_id})
                     MERGE (h)-[:RUNS_SERVICE]->(s)",
                )
                .param("host", host.clone())
                .param("svc_id", name.clone()))
                .await?;
            stats.relations += 1;
        }

        // Sync containers
        for (host, name, image, status) in &data.containers {
            self.graph
                .run(query(
                    "MERGE (c:Container {id: $id})
                     SET c.name = $name, c.image = $image, c.status = $status",
                )
                .param("id", name.clone())
                .param("name", name.clone())
                .param("image", image.clone())
                .param("status", status.clone()))
                .await?;
            stats.nodes += 1;

            // Link container to host
            self.graph
                .run(query(
                    "MATCH (h:Host {hostname: $host}), (c:Container {id: $ctr_id})
                     MERGE (h)-[:RUNS_CONTAINER]->(c)",
                )
                .param("host", host.clone())
                .param("ctr_id", name.clone()))
                .await?;
            stats.relations += 1;
        }

        // Sync ports
        for (_host, protocol, port, process) in &data.ports {
            let port_id = format!("{}:{}", protocol, port);
            self.graph
                .run(query(
                    "MERGE (pt:Port {id: $id})
                     SET pt.protocol = $protocol, pt.port = $port, pt.process = $process",
                )
                .param("id", port_id.clone())
                .param("protocol", protocol.clone())
                .param("port", *port as i64)
                .param("process", process.clone()))
                .await?;
            stats.nodes += 1;
        }

        // Sync dependencies as relationships
        for dep in &data.dependencies {
            let cypher = format!(
                "MERGE (a:{} {{id: $source_id}})
                 MERGE (b:{} {{id: $target_id}})
                 MERGE (a)-[r:{}]->(b)
                 SET r.confidence = $confidence, r.origin = $origin",
                dep.source_type, dep.target_type, dep.relation_type
            );

            self.graph
                .run(query(&cypher)
                    .param("source_id", dep.source_id.clone())
                    .param("target_id", dep.target_id.clone())
                    .param("confidence", dep.confidence)
                    .param("origin", dep.origin.clone()))
                .await?;
            stats.relations += 1;
        }

        info!(nodes = stats.nodes, relations = stats.relations, "Neo4j sync complete");
        Ok(stats)
    }

    /// Get downstream impact tree from a node.
    pub async fn get_impact_downstream(&self, node_type: &str, node_id: &str) -> Result<Vec<(String, String, String, i64)>> {
        let cypher = format!(
            "MATCH path = (source:{} {{id: $id}})-[:DEPENDS_ON|LISTENS_ON*1..5]-(target)
             RETURN labels(target)[0] AS target_type,
                    target.id AS target_id,
                    type(relationships(path)[-1]) AS rel_type,
                    length(path) AS depth
             ORDER BY depth ASC",
            node_type
        );

        let mut result = self.graph
            .execute(query(&cypher).param("id", node_id.to_string()))
            .await?;

        let mut nodes = Vec::new();
        while let Some(row) = result.next().await? {
            let target_type: String = row.get("target_type").unwrap_or_default();
            let target_id: String = row.get("target_id").unwrap_or_default();
            let rel_type: String = row.get("rel_type").unwrap_or_default();
            let depth: i64 = row.get("depth").unwrap_or_default();
            nodes.push((target_type, target_id, rel_type, depth));
        }

        Ok(nodes)
    }

    /// Find shortest path between two nodes.
    pub async fn find_shortest_path(&self, from_type: &str, from_id: &str, to_type: &str, to_id: &str) -> Result<Option<Vec<String>>> {
        let cypher = format!(
            "MATCH (a:{} {{id: $from_id}}), (b:{} {{id: $to_id}}),
                   path = shortestPath((a)-[*..15]-(b))
             RETURN [node IN nodes(path) | labels(node)[0] + ':' + node.id] AS path_nodes",
            from_type, to_type
        );

        let mut result = self.graph
            .execute(query(&cypher)
                .param("from_id", from_id.to_string())
                .param("to_id", to_id.to_string()))
            .await?;

        if let Some(row) = result.next().await? {
            let path_nodes: Vec<String> = row.get("path_nodes").unwrap_or_default();
            Ok(Some(path_nodes))
        } else {
            Ok(None)
        }
    }

    /// Get upstream nodes (candidates for root cause).
    pub async fn get_upstream_candidates(&self, node_type: &str, node_id: &str) -> Result<Vec<(String, String, String, i64, f64)>> {
        let cypher = format!(
            "MATCH path = (target:{} {{id: $id}})-[*1..10]->(candidate)
             WITH candidate, path,
                  labels(candidate)[0] AS candidate_type,
                  candidate.id AS candidate_id,
                  length(path) AS depth
             OPTIONAL MATCH (target:{} {{id: $id}})-[r]->(candidate)
             RETURN candidate_type,
                    candidate_id,
                    COALESCE(candidate.active_state, candidate.status, 'unknown') AS status,
                    depth,
                    COALESCE(r.confidence, 0.5) AS confidence
             ORDER BY depth ASC",
            node_type, node_type
        );

        let mut result = self.graph
            .execute(query(&cypher).param("id", node_id.to_string()))
            .await?;

        let mut candidates = Vec::new();
        while let Some(row) = result.next().await? {
            let candidate_type: String = row.get("candidate_type").unwrap_or_default();
            let candidate_id: String = row.get("candidate_id").unwrap_or_default();
            let status: String = row.get("status").unwrap_or_default();
            let depth: i64 = row.get("depth").unwrap_or_default();
            let confidence: f64 = row.get("confidence").unwrap_or_default();
            candidates.push((candidate_type, candidate_id, status, depth, confidence));
        }

        Ok(candidates)
    }

    /// Get the path from a candidate back to the affected node.
    pub async fn get_path_to_affected(&self, candidate_type: &str, candidate_id: &str, affected_type: &str, affected_id: &str) -> Result<Vec<String>> {
        let cypher = format!(
            "MATCH (a:{} {{id: $from_id}}), (b:{} {{id: $to_id}}),
                   path = shortestPath((a)-[*..15]->(b))
             RETURN [node IN nodes(path) | labels(node)[0] + ':' + node.id] AS path_nodes",
            candidate_type, affected_type
        );

        let mut result = self.graph
            .execute(query(&cypher)
                .param("from_id", candidate_id.to_string())
                .param("to_id", affected_id.to_string()))
            .await?;

        if let Some(row) = result.next().await? {
            let path_nodes: Vec<String> = row.get("path_nodes").unwrap_or_default();
            Ok(path_nodes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Count downstream dependencies for a node (for scoring).
    pub async fn count_downstream(&self, node_type: &str, node_id: &str) -> Result<i64> {
        let cypher = format!(
            "MATCH (n:{} {{id: $id}})-[*1..10]->(target)
             RETURN count(DISTINCT target) AS cnt",
            node_type
        );

        let mut result = self.graph
            .execute(query(&cypher).param("id", node_id.to_string()))
            .await?;

        if let Some(row) = result.next().await? {
            let cnt: i64 = row.get("cnt").unwrap_or_default();
            Ok(cnt)
        } else {
            Ok(0)
        }
    }
}
