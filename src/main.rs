use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod cli;
mod config;
mod models;
mod collectors;
mod storage;
mod graph;
mod analysis;
mod output;
mod report;

use cli::{Cli, Commands, IncidentCommands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("rootmap=info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Scan => {
            info!("Starting infrastructure scan...");
            cmd_scan(&cfg).await?;
        }
        Commands::Import { file } => {
            info!("Importing dependencies from: {}", file);
            cmd_import(&cfg, &file).await?;
        }
        Commands::Sync => {
            info!("Syncing data to Neo4j...");
            cmd_sync(&cfg).await?;
        }
        Commands::Impact { node } => {
            cmd_impact(&cfg, &node).await?;
        }
        Commands::Path { from, to } => {
            cmd_path(&cfg, &from, &to).await?;
        }
        Commands::Incident(sub) => match sub {
            IncidentCommands::Analyze { symptom, affected } => {
                cmd_incident_analyze(&cfg, &symptom, &affected).await?;
            }
        },
        Commands::Report { incident, last, format } => {
            cmd_report(&cfg, incident, last, &format).await?;
        }
        Commands::Migrate => {
            info!("Running database migrations...");
            cmd_migrate(&cfg).await?;
        }
    }

    Ok(())
}

async fn cmd_scan(cfg: &config::Config) -> Result<()> {
    use collectors::{linux, systemd, docker, ports};
    use storage::sqlite::SqliteStorage;
    use output::table;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;

    // Create scan run
    let scan_id = pg.create_scan_run().await?;

    // Collect host info
    let host = linux::collect_host_info();
    let host_id = pg.upsert_host(&host).await?;

    // Collect processes
    let processes = linux::collect_processes();
    let important = linux::detect_important_processes(&processes);
    pg.insert_processes(&processes, scan_id, host_id).await?;

    // Collect systemd services
    let services = systemd::collect_services().await;
    pg.insert_services(&services, scan_id, host_id).await?;

    // Collect Docker containers
    let containers = docker::collect_containers().await;
    pg.insert_containers(&containers, scan_id, host_id).await?;

    // Collect listening ports
    let listening = ports::collect_ports().await;
    pg.insert_ports(&listening, scan_id, host_id).await?;

    // Auto-detect dependencies from scan data
    let auto_deps = collectors::detect_dependencies(&processes, &services, &containers, &listening);
    pg.insert_dependencies(&auto_deps).await?;

    // Complete scan run
    pg.finish_scan_run(scan_id, "completed").await?;

    // Print summary
    table::print_scan_summary(&host, &processes, &services, &containers, &listening, &important);

    Ok(())
}

async fn cmd_import(cfg: &config::Config, file: &str) -> Result<()> {
    use storage::sqlite::SqliteStorage;
    use models::InventoryFile;

    let content = std::fs::read_to_string(file)?;
    let inventory: InventoryFile = serde_yaml::from_str(&content)?;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;
    let count = pg.import_dependencies(&inventory.dependencies).await?;

    println!("✓ Imported {} dependencies from {}", count, file);
    Ok(())
}

async fn cmd_sync(cfg: &config::Config) -> Result<()> {
    use storage::sqlite::SqliteStorage;
    use graph::neo4j::Neo4jGraph;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;
    let neo = Neo4jGraph::connect(&cfg.neo4j_url, &cfg.neo4j_user, &cfg.neo4j_pass).await?;

    neo.ensure_constraints().await?;

    // Sync from SQLite to Neo4j
    let data = pg.get_sync_data().await?;
    let stats = neo.sync_all(&data).await?;

    println!("Neo4j sync complete:");
    println!("  Nodes created/updated: {}", stats.nodes);
    println!("  Relations created:     {}", stats.relations);

    Ok(())
}

async fn cmd_impact(cfg: &config::Config, node: &str) -> Result<()> {
    use graph::neo4j::Neo4jGraph;
    use analysis::impact;
    use output::{ascii, table};

    let neo = Neo4jGraph::connect(&cfg.neo4j_url, &cfg.neo4j_user, &cfg.neo4j_pass).await?;
    let tree = impact::analyze_impact(&neo, node).await?;

    println!("\n╔══════════════════════════════════════╗");
    println!("║       IMPACT ANALYSIS                ║");
    println!("╚══════════════════════════════════════╝\n");
    println!("Source node: {}\n", node);

    if tree.children.is_empty() {
        println!("No downstream dependencies found for this node.");
    } else {
        ascii::print_tree(&tree, 0);
        println!();
        table::print_impact_table(&tree);
    }

    Ok(())
}

async fn cmd_path(cfg: &config::Config, from: &str, to: &str) -> Result<()> {
    use graph::neo4j::Neo4jGraph;
    use analysis::path;
    use output::table;

    let neo = Neo4jGraph::connect(&cfg.neo4j_url, &cfg.neo4j_user, &cfg.neo4j_pass).await?;
    let result = path::find_path(&neo, from, to).await?;

    println!("\n╔══════════════════════════════════════╗");
    println!("║       PATH ANALYSIS                  ║");
    println!("╚══════════════════════════════════════╝\n");
    println!("From: {}", from);
    println!("To:   {}\n", to);

    match result {
        Some(p) => {
            let path_str = p.join(" → ");
            println!("Path: {}\n", path_str);
            table::print_path_table(&p);
        }
        None => {
            println!("No path found between {} and {}", from, to);
        }
    }

    Ok(())
}

async fn cmd_incident_analyze(cfg: &config::Config, symptom: &str, affected: &str) -> Result<()> {
    use graph::neo4j::Neo4jGraph;
    use storage::sqlite::SqliteStorage;
    use analysis::incident;
    use output::table;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;
    let neo = Neo4jGraph::connect(&cfg.neo4j_url, &cfg.neo4j_user, &cfg.neo4j_pass).await?;

    let result = incident::analyze(&neo, symptom, affected).await?;

    // Store incident and findings in SQLite
    let incident_id = pg.insert_incident(symptom, affected).await?;
    pg.insert_findings(incident_id, &result.candidates).await?;

    println!("\n╔══════════════════════════════════════╗");
    println!("║   INCIDENT ANALYSIS (HEURISTIC)      ║");
    println!("╚══════════════════════════════════════╝\n");
    println!("Symptom:  {}", symptom);
    println!("Affected: {}\n", affected);
    println!("⚠  The results below are HYPOTHESES based on heuristic");
    println!("   analysis of known dependencies. They are NOT definitive");
    println!("   root cause determinations.\n");

    if result.candidates.is_empty() {
        println!("No upstream candidates found for {}", affected);
    } else {
        table::print_incident_candidates(&result.candidates);
        println!("\nIncident stored with ID: {}", incident_id);
    }

    Ok(())
}

async fn cmd_report(cfg: &config::Config, incident_id: Option<String>, last: bool, format: &str) -> Result<()> {
    use storage::sqlite::SqliteStorage;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;

    let id = if last {
        pg.get_last_incident_id().await?
    } else if let Some(ref id_str) = incident_id {
        Some(uuid::Uuid::parse_str(id_str)?)
    } else {
        anyhow::bail!("Specify --incident <id> or --last");
    };

    match id {
        Some(id) => {
            let report_content = report::generate_report(&pg, id, format).await?;
            println!("{}", report_content);
        }
        None => {
            println!("No incidents found.");
        }
    }

    Ok(())
}

async fn cmd_migrate(cfg: &config::Config) -> Result<()> {
    use storage::sqlite::SqliteStorage;

    let pg = SqliteStorage::connect(&cfg.db_url).await?;
    pg.run_migrations().await?;
    println!("✓ Migrations completed successfully.");
    Ok(())
}
