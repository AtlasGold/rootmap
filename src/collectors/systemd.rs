use crate::models::SystemdService;
use tokio::process::Command;
use tracing::{info, warn};

/// Collect systemd services by running `systemctl list-units`.
/// Returns empty vec if systemctl is not available.
pub async fn collect_services() -> Vec<SystemdService> {
    let output = Command::new("systemctl")
        .args(["list-units", "--type=service", "--all", "--no-pager", "--no-legend"])
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                warn!("systemctl returned non-zero exit code. Systemd services will not be collected.");
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&out.stdout);
            let services = parse_systemctl_output(&stdout);
            info!(count = services.len(), "Systemd services collected");
            services
        }
        Err(e) => {
            warn!("systemctl not available ({}). Systemd services will not be collected.", e);
            Vec::new()
        }
    }
}

/// Parse the output of `systemctl list-units --type=service --all --no-legend`
fn parse_systemctl_output(output: &str) -> Vec<SystemdService> {
    let mut services = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: UNIT LOAD ACTIVE SUB DESCRIPTION...
        // The unit may have a ● prefix for failed units
        let line = line.trim_start_matches('●').trim();

        let parts: Vec<&str> = line.splitn(5, char::is_whitespace).collect();
        if parts.len() < 4 {
            continue;
        }

        // Filter out non-empty parts (handle multiple spaces)
        let mut fields = Vec::new();
        let mut rest = line;
        for _ in 0..4 {
            rest = rest.trim_start();
            if let Some(space_pos) = rest.find(char::is_whitespace) {
                fields.push(&rest[..space_pos]);
                rest = &rest[space_pos..];
            } else {
                fields.push(rest);
                rest = "";
                break;
            }
        }
        let description = rest.trim().to_string();

        if fields.len() < 4 {
            continue;
        }

        services.push(SystemdService {
            name: fields[0].to_string(),
            load_state: fields[1].to_string(),
            active_state: fields[2].to_string(),
            sub_state: fields[3].to_string(),
            description,
        });
    }

    services
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_systemctl_output() {
        let output = r#"  nginx.service                loaded active running A high performance web server
  postgresql.service           loaded active running PostgreSQL RDBMS
● failed-svc.service           loaded failed failed  Some failed service
  ssh.service                  loaded active running OpenBSD Secure Shell server
"#;

        let services = parse_systemctl_output(output);
        assert_eq!(services.len(), 4);
        assert_eq!(services[0].name, "nginx.service");
        assert_eq!(services[0].active_state, "active");
        assert_eq!(services[2].name, "failed-svc.service");
        assert_eq!(services[2].active_state, "failed");
    }
}
