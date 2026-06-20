use comfy_table::{Table, Cell, Color, Attribute, ContentArrangement};

use crate::models::*;

/// Print a summary table after a scan.
pub fn print_scan_summary(
    host: &Host,
    processes: &[ProcessInfo],
    services: &[SystemdService],
    containers: &[DockerContainer],
    ports: &[ListeningPort],
    important: &[String],
) {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║               ROOTMAP — SCAN RESULTS                   ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Host info table
    let mut host_table = Table::new();
    host_table.set_content_arrangement(ContentArrangement::Dynamic);
    host_table.set_header(vec![
        Cell::new("Property").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    host_table.add_row(vec!["Hostname", &host.hostname]);
    host_table.add_row(vec!["OS", &host.os]);
    host_table.add_row(vec!["Kernel", &host.kernel]);
    println!("🖥  Host Information");
    println!("{}\n", host_table);

    // Summary table
    let mut summary = Table::new();
    summary.set_content_arrangement(ContentArrangement::Dynamic);
    summary.set_header(vec![
        Cell::new("Category").add_attribute(Attribute::Bold),
        Cell::new("Count").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ]);

    summary.add_row(vec![
        Cell::new("Processes"),
        Cell::new(processes.len()),
        Cell::new("✓ collected"),
    ]);

    let active_services = services.iter().filter(|s| s.active_state == "active").count();
    let failed_services = services.iter().filter(|s| s.active_state == "failed").count();
    let svc_status = if services.is_empty() {
        "⚠ systemctl not available".to_string()
    } else {
        format!("✓ {} active, {} failed", active_services, failed_services)
    };
    summary.add_row(vec![
        Cell::new("Systemd Services"),
        Cell::new(services.len()),
        Cell::new(&svc_status),
    ]);

    let ctr_status = if containers.is_empty() {
        "⚠ Docker not available or no containers".to_string()
    } else {
        "✓ collected".to_string()
    };
    summary.add_row(vec![
        Cell::new("Docker Containers"),
        Cell::new(containers.len()),
        Cell::new(&ctr_status),
    ]);

    let port_status = if ports.is_empty() {
        "⚠ ss not available or no ports".to_string()
    } else {
        "✓ collected".to_string()
    };
    summary.add_row(vec![
        Cell::new("Listening Ports"),
        Cell::new(ports.len()),
        Cell::new(&port_status),
    ]);

    println!("📊 Scan Summary");
    println!("{}\n", summary);

    // Important components
    if !important.is_empty() {
        let mut imp_table = Table::new();
        imp_table.set_content_arrangement(ContentArrangement::Dynamic);
        imp_table.set_header(vec![
            Cell::new("Component").add_attribute(Attribute::Bold),
            Cell::new("Detected As").add_attribute(Attribute::Bold),
        ]);
        for name in important {
            let detected_as = match name.as_str() {
                "nginx" => "Web Server / Reverse Proxy",
                "postgres" | "postmaster" => "PostgreSQL Database",
                "docker" | "containerd" => "Container Runtime",
                "sshd" => "SSH Server",
                "mysqld" => "MySQL Database",
                "redis-server" => "Redis Cache",
                "mongod" => "MongoDB Database",
                "node" => "Node.js Application",
                "apache2" | "httpd" => "Apache Web Server",
                "haproxy" => "HAProxy Load Balancer",
                _ => "Service",
            };
            imp_table.add_row(vec![name.as_str(), detected_as]);
        }
        println!("🔍 Important Components Detected");
        println!("{}\n", imp_table);
    }

    println!("Scan data persisted successfully.");
}

/// Print impact analysis as a table.
pub fn print_impact_table(tree: &ImpactNode) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Node").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Depth").add_attribute(Attribute::Bold),
    ]);

    for (node_str, depth) in tree.flatten() {
        if depth == 0 {
            table.add_row(vec![
                Cell::new(&node_str).add_attribute(Attribute::Bold).fg(Color::Cyan),
                Cell::new("SOURCE"),
                Cell::new(depth),
            ]);
        } else {
            let indent = "  ".repeat(depth);
            table.add_row(vec![
                Cell::new(format!("{}{}", indent, node_str)),
                Cell::new(if depth == 1 { "DIRECT" } else { "INDIRECT" }),
                Cell::new(depth),
            ]);
        }
    }

    println!("Impact Table:");
    println!("{}", table);
    println!("\nTotal potentially impacted: {} nodes", tree.count_all() - 1);
}

/// Print path between nodes as a table.
pub fn print_path_table(path: &[String]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Step").add_attribute(Attribute::Bold),
        Cell::new("Node").add_attribute(Attribute::Bold),
    ]);

    for (i, node) in path.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(node),
        ]);
    }

    println!("{}", table);
    println!("\nPath length: {} hops", path.len() - 1);
}

/// Print incident analysis candidates.
pub fn print_incident_candidates(candidates: &[IncidentCandidate]) {
    println!("Probable cause candidates:\n");

    for (i, c) in candidates.iter().enumerate() {
        println!(
            "  {}. {} — score={:.2}",
            i + 1,
            c.node,
            c.score
        );
        println!("     Reason: {}", c.reason);
        println!("     Path:   {}", c.path.join(" → "));
        println!();
    }

    // Also as a compact table
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Candidate").add_attribute(Attribute::Bold),
        Cell::new("Score").add_attribute(Attribute::Bold),
        Cell::new("Reason").add_attribute(Attribute::Bold),
    ]);

    for (i, c) in candidates.iter().enumerate() {
        let score_cell = if c.score >= 0.7 {
            Cell::new(format!("{:.2}", c.score)).fg(Color::Red).add_attribute(Attribute::Bold)
        } else if c.score >= 0.4 {
            Cell::new(format!("{:.2}", c.score)).fg(Color::Yellow)
        } else {
            Cell::new(format!("{:.2}", c.score))
        };

        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(&c.node),
            score_cell,
            Cell::new(&c.reason),
        ]);
    }

    println!("{}", table);
}
