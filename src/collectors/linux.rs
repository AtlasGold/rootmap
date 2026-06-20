use crate::models::{Host, ProcessInfo};
use sysinfo::System;
use tracing::info;

/// Collect host information (hostname, OS, kernel).
pub fn collect_host_info() -> Host {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os = format!(
        "{} {}",
        System::name().unwrap_or_else(|| "Linux".to_string()),
        System::os_version().unwrap_or_default()
    );
    let kernel = System::kernel_version().unwrap_or_else(|| "unknown".to_string());

    info!(hostname = %hostname, os = %os, kernel = %kernel, "Host info collected");

    Host {
        hostname,
        os,
        kernel,
    }
}

/// Collect all running processes via sysinfo.
pub fn collect_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut processes = Vec::new();

    for (pid, proc_info) in sys.processes() {
        let ppid = proc_info.parent().map(|p| p.as_u32());

        let cmd_str = proc_info
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        processes.push(ProcessInfo {
            pid: pid.as_u32(),
            ppid,
            name: proc_info.name().to_string_lossy().to_string(),
            command: cmd_str,
            user_name: proc_info
                .user_id()
                .map(|uid| uid.to_string())
                .unwrap_or_default(),
            status: format!("{:?}", proc_info.status()),
        });
    }

    info!(count = processes.len(), "Processes collected");
    processes
}

/// Detect important/relevant processes from the collected list.
pub fn detect_important_processes(processes: &[ProcessInfo]) -> Vec<String> {
    let important_names = [
        "nginx", "postgres", "postmaster", "docker", "containerd",
        "sshd", "mysqld", "redis-server", "mongod", "node",
        "apache2", "httpd", "haproxy",
    ];

    let mut found = Vec::new();

    for name in &important_names {
        if processes.iter().any(|p| p.name.to_lowercase().contains(name)) {
            found.push(name.to_string());
        }
    }

    found
}
