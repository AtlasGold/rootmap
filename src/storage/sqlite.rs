use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool, Row};
use uuid::Uuid;
use tracing::info;

use crate::models::*;

/// SQLite storage layer for rootmap.
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Connect to SQLite.
    pub async fn connect(url: &str) -> Result<Self> {
        // Ensure connection URL has correct format
        let connection_url = if !url.starts_with("sqlite:") {
            format!("sqlite://{}", url)
        } else {
            url.to_string()
        };

        let options = connection_url
            .parse::<SqliteConnectOptions>()
            .context("Failed to parse SQLite connection URL")?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .context("Failed to connect to SQLite database")?;

        let storage = SqliteStorage { pool };
        
        // Execute automatic migrations
        storage.run_migrations().await.context("Failed to auto-run SQLite migrations")?;

        Ok(storage)
    }

    /// Run database migrations.
    pub async fn run_migrations(&self) -> Result<()> {
        let sql = include_str!("../../migrations/001_init.sql");
        sqlx::raw_sql(sql)
            .execute(&self.pool)
            .await
            .context("Failed to run migrations")?;
        Ok(())
    }

    // ─── Scan Runs ──────────────────────────────────────────────────

    pub async fn create_scan_run(&self) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let status = "running".to_string();
        
        sqlx::query("INSERT INTO scan_runs (id, started_at, status) VALUES (?, ?, ?)")
            .bind(id)
            .bind(now)
            .bind(status)
            .execute(&self.pool)
            .await?;
            
        Ok(id)
    }

    pub async fn finish_scan_run(&self, id: Uuid, status: &str) -> Result<()> {
        let now = Utc::now();
        let status_owned = status.to_string();
        
        sqlx::query("UPDATE scan_runs SET finished_at = ?, status = ? WHERE id = ?")
            .bind(now)
            .bind(status_owned)
            .bind(id)
            .execute(&self.pool)
            .await?;
            
        Ok(())
    }

    // ─── Hosts ──────────────────────────────────────────────────────

    pub async fn upsert_host(&self, host: &Host) -> Result<Uuid> {
        let row = sqlx::query("SELECT id FROM hosts WHERE hostname = ?")
            .bind(&host.hostname)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            let id: Uuid = r.get(0);
            let now = Utc::now();
            
            sqlx::query("UPDATE hosts SET os = ?, kernel = ?, updated_at = ? WHERE id = ?")
                .bind(&host.os)
                .bind(&host.kernel)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
                
            Ok(id)
        } else {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO hosts (id, hostname, os, kernel) VALUES (?, ?, ?, ?)")
                .bind(id)
                .bind(&host.hostname)
                .bind(&host.os)
                .bind(&host.kernel)
                .execute(&self.pool)
                .await?;
                
            Ok(id)
        }
    }

    // ─── Processes ──────────────────────────────────────────────────

    pub async fn insert_processes(
        &self,
        processes: &[ProcessInfo],
        scan_id: Uuid,
        host_id: Uuid,
    ) -> Result<()> {
        for proc in processes {
            let id = Uuid::new_v4();
            let pid = proc.pid as i32;
            let ppid = proc.ppid.map(|p| p as i32);
            
            sqlx::query(
                "INSERT INTO processes (id, scan_run_id, host_id, pid, ppid, name, command, user_name, status)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(scan_id)
            .bind(host_id)
            .bind(pid)
            .bind(ppid)
            .bind(&proc.name)
            .bind(&proc.command)
            .bind(&proc.user_name)
            .bind(&proc.status)
            .execute(&self.pool)
            .await?;
        }
        info!(count = processes.len(), "Processes stored");
        Ok(())
    }

    // ─── Systemd Services ───────────────────────────────────────────

    pub async fn insert_services(
        &self,
        services: &[SystemdService],
        scan_id: Uuid,
        host_id: Uuid,
    ) -> Result<()> {
        for svc in services {
            let id = Uuid::new_v4();
            
            sqlx::query(
                "INSERT INTO systemd_services (id, scan_run_id, host_id, name, load_state, active_state, sub_state, description)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(scan_id)
            .bind(host_id)
            .bind(&svc.name)
            .bind(&svc.load_state)
            .bind(&svc.active_state)
            .bind(&svc.sub_state)
            .bind(&svc.description)
            .execute(&self.pool)
            .await?;
        }
        info!(count = services.len(), "Systemd services stored");
        Ok(())
    }

    // ─── Docker Containers ──────────────────────────────────────────

    pub async fn insert_containers(
        &self,
        containers: &[DockerContainer],
        scan_id: Uuid,
        host_id: Uuid,
    ) -> Result<()> {
        for ctr in containers {
            let id = Uuid::new_v4();
            
            sqlx::query(
                "INSERT INTO docker_containers (id, scan_run_id, host_id, container_id, name, image, status, ports)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(scan_id)
            .bind(host_id)
            .bind(&ctr.container_id)
            .bind(&ctr.name)
            .bind(&ctr.image)
            .bind(&ctr.status)
            .bind(&ctr.ports)
            .execute(&self.pool)
            .await?;
        }
        info!(count = containers.len(), "Docker containers stored");
        Ok(())
    }

    // ─── Listening Ports ────────────────────────────────────────────

    pub async fn insert_ports(
        &self,
        ports: &[ListeningPort],
        scan_id: Uuid,
        host_id: Uuid,
    ) -> Result<()> {
        for port in ports {
            let id = Uuid::new_v4();
            let port_num = port.port as i32;
            let pid = port.pid.map(|p| p as i32);
            
            sqlx::query(
                "INSERT INTO listening_ports (id, scan_run_id, host_id, protocol, local_address, port, process_name, pid)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(scan_id)
            .bind(host_id)
            .bind(&port.protocol)
            .bind(&port.local_address)
            .bind(port_num)
            .bind(&port.process_name)
            .bind(pid)
            .execute(&self.pool)
            .await?;
        }
        info!(count = ports.len(), "Listening ports stored");
        Ok(())
    }

    // ─── Dependencies ───────────────────────────────────────────────

    pub async fn insert_dependencies(&self, deps: &[Dependency]) -> Result<()> {
        for dep in deps {
            let id = Uuid::new_v4();
            
            sqlx::query(
                "INSERT INTO dependencies (id, source_type, source_id, target_type, target_id, relation_type, confidence, origin)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(&dep.source_type)
            .bind(&dep.source_id)
            .bind(&dep.target_type)
            .bind(&dep.target_id)
            .bind(&dep.relation_type)
            .bind(dep.confidence)
            .bind(&dep.origin)
            .execute(&self.pool)
            .await?;
        }
        info!(count = deps.len(), "Dependencies stored");
        Ok(())
    }

    pub async fn import_dependencies(&self, deps: &[Dependency]) -> Result<usize> {
        let mut count = 0;
        for dep in deps {
            let id = Uuid::new_v4();
            let origin = "manual".to_string();
            
            sqlx::query(
                "INSERT INTO dependencies (id, source_type, source_id, target_type, target_id, relation_type, confidence, origin)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(&dep.source_type)
            .bind(&dep.source_id)
            .bind(&dep.target_type)
            .bind(&dep.target_id)
            .bind(&dep.relation_type)
            .bind(dep.confidence)
            .bind(origin)
            .execute(&self.pool)
            .await?;
            
            count += 1;
        }
        Ok(count)
    }

    // ─── Sync Data (for Neo4j) ──────────────────────────────────────

    pub async fn get_sync_data(&self) -> Result<SyncData> {
        let mut data = SyncData::default();

        // Get hosts
        let rows = sqlx::query("SELECT id, hostname, os, kernel FROM hosts")
            .fetch_all(&self.pool)
            .await?;
            
        for row in rows {
            let id: Uuid = row.get(0);
            data.hosts.push((
                id,
                row.get(1),
                row.get::<Option<String>, _>(2).unwrap_or_default(),
                row.get::<Option<String>, _>(3).unwrap_or_default(),
            ));
        }

        // Get latest scan processes (only important ones)
        let rows = sqlx::query(
            "SELECT h.hostname, p.name, p.pid, p.status
             FROM processes p
             JOIN hosts h ON p.host_id = h.id
             WHERE p.scan_run_id = (SELECT id FROM scan_runs ORDER BY started_at DESC LIMIT 1)
             AND LOWER(p.name) IN ('nginx', 'postgres', 'postmaster', 'docker', 'containerd', 'sshd', 'mysqld', 'redis-server', 'mongod', 'node', 'apache2', 'httpd', 'haproxy')"
        )
        .fetch_all(&self.pool)
        .await?;
        
        for row in rows {
            let pid: i32 = row.get(2);
            data.processes.push((
                row.get(0),
                row.get(1),
                pid as u32,
                row.get::<Option<String>, _>(3).unwrap_or_default(),
            ));
        }

        // Get latest scan services
        let rows = sqlx::query(
            "SELECT h.hostname, s.name, s.active_state, s.sub_state
             FROM systemd_services s
             JOIN hosts h ON s.host_id = h.id
             WHERE s.scan_run_id = (SELECT id FROM scan_runs ORDER BY started_at DESC LIMIT 1)"
        )
        .fetch_all(&self.pool)
        .await?;
        
        for row in rows {
            data.services.push((
                row.get(0),
                row.get::<String, _>(1).replace(".service", ""),
                row.get::<Option<String>, _>(2).unwrap_or_default(),
                row.get::<Option<String>, _>(3).unwrap_or_default(),
            ));
        }

        // Get latest scan containers
        let rows = sqlx::query(
            "SELECT h.hostname, c.name, c.image, c.status
             FROM docker_containers c
             JOIN hosts h ON c.host_id = h.id
             WHERE c.scan_run_id = (SELECT id FROM scan_runs ORDER BY started_at DESC LIMIT 1)"
        )
        .fetch_all(&self.pool)
        .await?;
        
        for row in rows {
            data.containers.push((
                row.get(0),
                row.get(1),
                row.get::<Option<String>, _>(2).unwrap_or_default(),
                row.get::<Option<String>, _>(3).unwrap_or_default(),
            ));
        }

        // Get latest scan ports
        let rows = sqlx::query(
            "SELECT h.hostname, lp.protocol, lp.port, lp.process_name
             FROM listening_ports lp
             JOIN hosts h ON lp.host_id = h.id
             WHERE lp.scan_run_id = (SELECT id FROM scan_runs ORDER BY started_at DESC LIMIT 1)"
        )
        .fetch_all(&self.pool)
        .await?;
        
        for row in rows {
            let port: i32 = row.get(2);
            data.ports.push((
                row.get(0),
                row.get(1),
                port as u16,
                row.get::<Option<String>, _>(3).unwrap_or_default(),
            ));
        }

        // Get all dependencies
        let rows = sqlx::query(
            "SELECT source_type, source_id, target_type, target_id, relation_type, confidence, origin
             FROM dependencies"
        )
        .fetch_all(&self.pool)
        .await?;
        
        for row in rows {
            data.dependencies.push(Dependency {
                source_type: row.get(0),
                source_id: row.get(1),
                target_type: row.get(2),
                target_id: row.get(3),
                relation_type: row.get(4),
                confidence: row.get(5),
                origin: row.get::<Option<String>, _>(6).unwrap_or_else(|| "unknown".to_string()),
            });
        }

        Ok(data)
    }

    // ─── Incidents ──────────────────────────────────────────────────

    pub async fn insert_incident(&self, symptom: &str, affected: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let title = format!("Incident: {} on {}", symptom, affected);
        let symptom_owned = symptom.to_string();
        let affected_owned = affected.to_string();
        
        sqlx::query("INSERT INTO incidents (id, title, symptom, affected_node) VALUES (?, ?, ?, ?)")
            .bind(id)
            .bind(title)
            .bind(symptom_owned)
            .bind(affected_owned)
            .execute(&self.pool)
            .await?;
            
        Ok(id)
    }

    pub async fn insert_findings(&self, incident_id: Uuid, candidates: &[IncidentCandidate]) -> Result<()> {
        for c in candidates {
            let id = Uuid::new_v4();
            
            sqlx::query(
                "INSERT INTO incident_findings (id, incident_id, candidate_node, score, reason)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(incident_id)
            .bind(&c.node)
            .bind(c.score)
            .bind(&c.reason)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_last_incident_id(&self) -> Result<Option<Uuid>> {
        let row = sqlx::query("SELECT id FROM incidents ORDER BY created_at DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(r) = row {
            let id: Uuid = r.get(0);
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    pub async fn get_incident(&self, id: Uuid) -> Result<Option<StoredIncident>> {
        let row = sqlx::query("SELECT id, title, symptom, affected_node, created_at FROM incidents WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(r) = row {
            let id: Uuid = r.get(0);
            let created_at_str: String = r.get(4);
            // Parse SQLite timestamp
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
                
            Ok(Some(StoredIncident {
                id,
                title: r.get(1),
                symptom: r.get(2),
                affected_node: r.get(3),
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_findings(&self, incident_id: Uuid) -> Result<Vec<StoredFinding>> {
        let rows = sqlx::query(
            "SELECT candidate_node, score, reason FROM incident_findings
             WHERE incident_id = ? ORDER BY score DESC"
        )
        .bind(incident_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows
            .iter()
            .map(|row| StoredFinding {
                candidate_node: row.get(0),
                score: row.get(1),
                reason: row.get(2),
            })
            .collect())
    }
}
