use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Host ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Host {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
}

// ─── Process ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub command: String,
    pub user_name: String,
    pub status: String,
}

// ─── Systemd Service ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SystemdService {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
}

// ─── Docker Container ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainer {
    #[serde(alias = "ID")]
    pub container_id: String,
    #[serde(alias = "Names")]
    pub name: String,
    #[serde(alias = "Image")]
    pub image: String,
    #[serde(alias = "Status")]
    pub status: String,
    #[serde(alias = "Ports")]
    pub ports: String,
}

// ─── Listening Port ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ListeningPort {
    pub protocol: String,
    pub local_address: String,
    pub port: u16,
    pub process_name: String,
    pub pid: Option<u32>,
}

// ─── Dependency ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_origin")]
    pub origin: String,
}

fn default_confidence() -> f64 {
    1.0
}

fn default_origin() -> String {
    "manual".to_string()
}

// ─── Inventory File (YAML import) ───────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct InventoryFile {
    pub host: String,
    pub dependencies: Vec<Dependency>,
}

// ─── Sync Data (PG → Neo4j) ────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SyncData {
    pub hosts: Vec<(Uuid, String, String, String)>,           // id, hostname, os, kernel
    pub processes: Vec<(String, String, u32, String)>,         // host, name, pid, status
    pub services: Vec<(String, String, String, String)>,       // host, name, active_state, sub_state
    pub containers: Vec<(String, String, String, String)>,     // host, name, image, status
    pub ports: Vec<(String, String, u16, String)>,             // host, protocol, port, process
    pub dependencies: Vec<Dependency>,
}

// ─── Sync Stats ─────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SyncStats {
    pub nodes: usize,
    pub relations: usize,
}

// ─── Impact Tree ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImpactNode {
    pub node_id: String,
    pub node_type: String,
    pub relation: String,
    pub children: Vec<ImpactNode>,
}

impl ImpactNode {
    pub fn leaf(node_id: String, node_type: String, relation: String) -> Self {
        Self {
            node_id,
            node_type,
            relation,
            children: Vec::new(),
        }
    }

    pub fn count_all(&self) -> usize {
        let mut count = 1;
        for child in &self.children {
            count += child.count_all();
        }
        count
    }

    pub fn flatten(&self) -> Vec<(String, usize)> {
        let mut result = vec![(format!("{}:{}", self.node_type, self.node_id), 0)];
        self.flatten_inner(&mut result, 1);
        result
    }

    fn flatten_inner(&self, result: &mut Vec<(String, usize)>, depth: usize) {
        for child in &self.children {
            result.push((format!("{}:{}", child.node_type, child.node_id), depth));
            child.flatten_inner(result, depth + 1);
        }
    }
}

// ─── Incident Analysis ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct IncidentCandidate {
    pub node: String,
    pub score: f64,
    pub reason: String,
    pub path: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct IncidentResult {
    pub symptom: String,
    pub affected: String,
    pub candidates: Vec<IncidentCandidate>,
}

// ─── Stored Incident (from PG) ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StoredIncident {
    pub id: Uuid,
    pub title: Option<String>,
    pub symptom: String,
    pub affected_node: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredFinding {
    pub candidate_node: String,
    pub score: f64,
    pub reason: Option<String>,
}
