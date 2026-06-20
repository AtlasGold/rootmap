use crate::models::ListeningPort;
use tokio::process::Command;
use tracing::{info, warn};

/// Collect listening ports using `ss -lntup`.
/// Falls back gracefully if ss is not available.
pub async fn collect_ports() -> Vec<ListeningPort> {
    let output = Command::new("ss")
        .args(["-lntup"])
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                warn!("ss command failed. Listening ports will not be collected.");
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&out.stdout);
            let ports = parse_ss_output(&stdout);
            info!(count = ports.len(), "Listening ports collected");
            ports
        }
        Err(e) => {
            warn!("ss not available ({}). Trying /proc/net/tcp fallback...", e);
            collect_from_proc().await
        }
    }
}

/// Parse the output of `ss -lntup`
fn parse_ss_output(output: &str) -> Vec<ListeningPort> {
    let mut ports = Vec::new();

    for line in output.lines().skip(1) {
        // Skip header
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Typical ss output columns:
        // Netid State Recv-Q Send-Q Local Address:Port Peer Address:Port Process
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let protocol = parts[0].to_string();
        let local = parts[4];

        // Parse address:port
        let (addr, port_num) = if let Some(last_colon) = local.rfind(':') {
            let addr = &local[..last_colon];
            let port_str = &local[last_colon + 1..];
            (
                addr.to_string(),
                port_str.parse::<u16>().unwrap_or(0),
            )
        } else {
            continue;
        };

        if port_num == 0 {
            continue;
        }

        // Extract process info from the last field if present
        let (process_name, pid) = if parts.len() >= 7 {
            parse_process_field(parts[6])
        } else {
            (String::new(), None)
        };

        ports.push(ListeningPort {
            protocol,
            local_address: addr,
            port: port_num,
            process_name,
            pid,
        });
    }

    ports
}

/// Parse process field from ss output like: users:(("nginx",pid=1234,fd=6))
fn parse_process_field(field: &str) -> (String, Option<u32>) {
    // Format: users:(("name",pid=NNN,fd=N))
    let mut name = String::new();
    let mut pid = None;

    if let Some(start) = field.find("((\"") {
        let rest = &field[start + 3..];
        if let Some(end) = rest.find('"') {
            name = rest[..end].to_string();
        }
    }

    if let Some(pid_start) = field.find("pid=") {
        let rest = &field[pid_start + 4..];
        let pid_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        pid = pid_str.parse().ok();
    }

    (name, pid)
}

/// Fallback: read from /proc/net/tcp
async fn collect_from_proc() -> Vec<ListeningPort> {
    match tokio::fs::read_to_string("/proc/net/tcp").await {
        Ok(content) => {
            let ports = parse_proc_net_tcp(&content);
            info!(count = ports.len(), "Listening ports collected from /proc/net/tcp");
            ports
        }
        Err(e) => {
            warn!("Cannot read /proc/net/tcp ({}). Listening ports will not be collected.", e);
            Vec::new()
        }
    }
}

/// Parse /proc/net/tcp format
fn parse_proc_net_tcp(content: &str) -> Vec<ListeningPort> {
    let mut ports = Vec::new();

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        // State 0A = LISTEN
        if parts[3] != "0A" {
            continue;
        }

        // local_address is hex IP:PORT
        let local = parts[1];
        if let Some(colon_pos) = local.find(':') {
            let hex_port = &local[colon_pos + 1..];
            if let Ok(port_num) = u16::from_str_radix(hex_port, 16) {
                let hex_addr = &local[..colon_pos];
                let addr = parse_hex_ip(hex_addr);

                ports.push(ListeningPort {
                    protocol: "tcp".to_string(),
                    local_address: addr,
                    port: port_num,
                    process_name: String::new(),
                    pid: None,
                });
            }
        }
    }

    ports
}

/// Convert hex IP (little-endian) to dotted notation
fn parse_hex_ip(hex: &str) -> String {
    if hex.len() != 8 {
        return hex.to_string();
    }
    if let Ok(ip_int) = u32::from_str_radix(hex, 16) {
        format!(
            "{}.{}.{}.{}",
            ip_int & 0xFF,
            (ip_int >> 8) & 0xFF,
            (ip_int >> 16) & 0xFF,
            (ip_int >> 24) & 0xFF,
        )
    } else {
        hex.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_process_field() {
        let field = r#"users:(("nginx",pid=1234,fd=6))"#;
        let (name, pid) = parse_process_field(field);
        assert_eq!(name, "nginx");
        assert_eq!(pid, Some(1234));
    }

    #[test]
    fn test_parse_hex_ip() {
        // 0.0.0.0
        assert_eq!(parse_hex_ip("00000000"), "0.0.0.0");
        // 127.0.0.1 in little-endian hex
        assert_eq!(parse_hex_ip("0100007F"), "127.0.0.1");
    }
}
