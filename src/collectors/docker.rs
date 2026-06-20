use crate::models::DockerContainer;
use tokio::process::Command;
use tracing::{info, warn};

/// Collect Docker containers by running `docker ps`.
/// Returns empty vec if Docker is not available or daemon is not running.
pub async fn collect_containers() -> Vec<DockerContainer> {
    // First try JSON format (newer Docker versions)
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "{{json .}}"])
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("Cannot connect") || stderr.contains("permission denied") {
                    warn!("Docker daemon not accessible. Containers will not be collected.");
                } else {
                    warn!("docker ps failed: {}. Containers will not be collected.", stderr.trim());
                }
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&out.stdout);
            let containers = parse_docker_json(&stdout);
            info!(count = containers.len(), "Docker containers collected");
            containers
        }
        Err(e) => {
            warn!("Docker not available ({}). Containers will not be collected.", e);
            Vec::new()
        }
    }
}

/// Parse JSON output from `docker ps --format '{{json .}}'`
fn parse_docker_json(output: &str) -> Vec<DockerContainer> {
    let mut containers = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(val) => {
                let container = DockerContainer {
                    container_id: val["ID"].as_str().unwrap_or("").to_string(),
                    name: val["Names"].as_str().unwrap_or("").to_string(),
                    image: val["Image"].as_str().unwrap_or("").to_string(),
                    status: val["Status"].as_str().unwrap_or("").to_string(),
                    ports: val["Ports"].as_str().unwrap_or("").to_string(),
                };
                containers.push(container);
            }
            Err(e) => {
                tracing::debug!("Failed to parse Docker JSON line: {}", e);
            }
        }
    }

    containers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_docker_json() {
        let output = r#"{"ID":"abc123","Names":"rootmap-postgres","Image":"postgres:16","Status":"Up 2 hours","Ports":"0.0.0.0:5432->5432/tcp"}
{"ID":"def456","Names":"rootmap-neo4j","Image":"neo4j:5","Status":"Up 2 hours","Ports":"0.0.0.0:7474->7474/tcp, 0.0.0.0:7687->7687/tcp"}"#;

        let containers = parse_docker_json(output);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0].name, "rootmap-postgres");
        assert_eq!(containers[1].name, "rootmap-neo4j");
    }
}
