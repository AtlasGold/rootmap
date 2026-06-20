use clap::{Parser, Subcommand};

/// rootmap — Linux infrastructure dependency mapper
///
/// Collects processes, systemd services, Docker containers, and open ports.
/// Stores data locally in SQLite and syncs relationships to Neo4j for
/// impact analysis, path finding, and incident root cause heuristics.
#[derive(Parser)]
#[command(name = "rootmap", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan local Linux infrastructure and store results in SQLite
    Scan,

    /// Import manual dependencies from a YAML file
    Import {
        /// Path to the YAML file
        #[arg(short, long)]
        file: String,
    },

    /// Sync data from SQLite to Neo4j
    Sync,

    /// Analyze impact downstream from a given node
    Impact {
        /// Node identifier (e.g., Service:postgresql)
        #[arg(long)]
        node: String,
    },

    /// Find the shortest path between two nodes
    Path {
        /// Source node (e.g., Service:nginx)
        #[arg(long)]
        from: String,

        /// Target node (e.g., Service:postgresql)
        #[arg(long)]
        to: String,
    },

    /// Incident analysis commands
    #[command(subcommand)]
    Incident(IncidentCommands),

    /// Generate an incident report
    Report {
        /// Incident UUID
        #[arg(long)]
        incident: Option<String>,

        /// Use the most recent incident
        #[arg(long, default_value_t = false)]
        last: bool,

        /// Output format: markdown or text
        #[arg(long, default_value = "markdown")]
        format: String,
    },

    /// Run database migrations
    Migrate,
}

#[derive(Subcommand)]
pub enum IncidentCommands {
    /// Analyze a symptom to find probable root causes
    Analyze {
        /// Description of the observed symptom
        #[arg(long)]
        symptom: String,

        /// The affected node (e.g., Service:nginx)
        #[arg(long)]
        affected: String,
    },
}
