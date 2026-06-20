pub mod linux;
pub mod systemd;
pub mod docker;
pub mod ports;

use crate::models::{ProcessInfo, SystemdService, DockerContainer, ListeningPort, Dependency};

/// Known important process names to detect
const IMPORTANT_PROCESSES: &[&str] = &[
    "nginx", "postgres", "postmaster", "docker", "containerd",
    "sshd", "mysqld", "redis-server", "mongod", "node",
    "java", "python3", "apache2", "httpd", "haproxy",
];

/// Automatically detect dependencies from scanned data.
/// This creates heuristic relationships based on what we found.
pub fn detect_dependencies(
    processes: &[ProcessInfo],
    services: &[SystemdService],
    _containers: &[DockerContainer],
    listening: &[ListeningPort],
) -> Vec<Dependency> {
    let mut deps = Vec::new();

    // Link services to their listening ports
    for port in listening {
        if port.process_name.is_empty() {
            continue;
        }

        let process_lower = port.process_name.to_lowercase();

        // Find matching service
        for svc in services {
            let svc_name_lower = svc.name.to_lowercase().replace(".service", "");
            if process_lower.contains(&svc_name_lower) || svc_name_lower.contains(&process_lower) {
                deps.push(Dependency {
                    source_type: "Service".to_string(),
                    source_id: svc.name.replace(".service", ""),
                    target_type: "Port".to_string(),
                    target_id: format!("{}:{}", port.protocol, port.port),
                    relation_type: "LISTENS_ON".to_string(),
                    confidence: 0.7,
                    origin: "auto-scan".to_string(),
                });
            }
        }

        // Find matching process
        for proc in processes {
            let proc_lower = proc.name.to_lowercase();
            if proc_lower == process_lower || process_lower.contains(&proc_lower) {
                deps.push(Dependency {
                    source_type: "Process".to_string(),
                    source_id: format!("{}:{}", proc.name, proc.pid),
                    target_type: "Port".to_string(),
                    target_id: format!("{}:{}", port.protocol, port.port),
                    relation_type: "LISTENS_ON".to_string(),
                    confidence: 0.6,
                    origin: "auto-scan".to_string(),
                });
                break;
            }
        }
    }

    // Link parent-child processes for important ones
    for proc in processes {
        if !IMPORTANT_PROCESSES.iter().any(|&n| proc.name.to_lowercase().contains(n)) {
            continue;
        }
        if let Some(ppid) = proc.ppid {
            if ppid > 1 {
                if let Some(parent) = processes.iter().find(|p| p.pid == ppid) {
                    deps.push(Dependency {
                        source_type: "Process".to_string(),
                        source_id: format!("{}:{}", parent.name, parent.pid),
                        target_type: "Process".to_string(),
                        target_id: format!("{}:{}", proc.name, proc.pid),
                        relation_type: "MANAGES_PROCESS".to_string(),
                        confidence: 0.8,
                        origin: "auto-scan".to_string(),
                    });
                }
            }
        }
    }

    deps
}
