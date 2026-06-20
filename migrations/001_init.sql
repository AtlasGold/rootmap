-- rootmap: Migration 001 - Initial Schema
-- SQLite schema for infrastructure dependency mapping

-- Scan runs track each execution of rootmap scan
CREATE TABLE IF NOT EXISTS scan_runs (
    id              TEXT PRIMARY KEY,
    started_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at     DATETIME,
    status          TEXT NOT NULL DEFAULT 'running'
);

-- Hosts represent scanned machines
CREATE TABLE IF NOT EXISTS hosts (
    id              TEXT PRIMARY KEY,
    hostname        TEXT NOT NULL,
    os              TEXT,
    kernel          TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hosts_hostname ON hosts(hostname);

-- Processes discovered during a scan
CREATE TABLE IF NOT EXISTS processes (
    id              TEXT PRIMARY KEY,
    scan_run_id     TEXT NOT NULL REFERENCES scan_runs(id),
    host_id         TEXT NOT NULL REFERENCES hosts(id),
    pid             INTEGER NOT NULL,
    ppid            INTEGER,
    name            TEXT NOT NULL,
    command         TEXT,
    user_name       TEXT,
    status          TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_processes_scan ON processes(scan_run_id);
CREATE INDEX IF NOT EXISTS idx_processes_host ON processes(host_id);

-- Systemd services discovered during a scan
CREATE TABLE IF NOT EXISTS systemd_services (
    id              TEXT PRIMARY KEY,
    scan_run_id     TEXT NOT NULL REFERENCES scan_runs(id),
    host_id         TEXT NOT NULL REFERENCES hosts(id),
    name            TEXT NOT NULL,
    load_state      TEXT,
    active_state    TEXT,
    sub_state       TEXT,
    description     TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_systemd_scan ON systemd_services(scan_run_id);
CREATE INDEX IF NOT EXISTS idx_systemd_host ON systemd_services(host_id);

-- Docker containers discovered during a scan
CREATE TABLE IF NOT EXISTS docker_containers (
    id              TEXT PRIMARY KEY,
    scan_run_id     TEXT NOT NULL REFERENCES scan_runs(id),
    host_id         TEXT NOT NULL REFERENCES hosts(id),
    container_id    TEXT NOT NULL,
    name            TEXT NOT NULL,
    image           TEXT,
    status          TEXT,
    ports           TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_docker_scan ON docker_containers(scan_run_id);
CREATE INDEX IF NOT EXISTS idx_docker_host ON docker_containers(host_id);

-- Listening ports discovered during a scan
CREATE TABLE IF NOT EXISTS listening_ports (
    id              TEXT PRIMARY KEY,
    scan_run_id     TEXT NOT NULL REFERENCES scan_runs(id),
    host_id         TEXT NOT NULL REFERENCES hosts(id),
    protocol        TEXT NOT NULL,
    local_address   TEXT,
    port            INTEGER NOT NULL,
    process_name    TEXT,
    pid             INTEGER,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ports_scan ON listening_ports(scan_run_id);
CREATE INDEX IF NOT EXISTS idx_ports_host ON listening_ports(host_id);

-- Dependencies between infrastructure components
CREATE TABLE IF NOT EXISTS dependencies (
    id              TEXT PRIMARY KEY,
    source_type     TEXT NOT NULL,
    source_id       TEXT NOT NULL,
    target_type     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    relation_type   TEXT NOT NULL,
    confidence      REAL DEFAULT 1.0,
    origin          TEXT DEFAULT 'manual',
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_deps_source ON dependencies(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_deps_target ON dependencies(target_type, target_id);

-- Incidents for root cause analysis
CREATE TABLE IF NOT EXISTS incidents (
    id              TEXT PRIMARY KEY,
    title           TEXT,
    symptom         TEXT NOT NULL,
    affected_node   TEXT NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Findings from incident analysis
CREATE TABLE IF NOT EXISTS incident_findings (
    id              TEXT PRIMARY KEY,
    incident_id     TEXT NOT NULL REFERENCES incidents(id),
    candidate_node  TEXT NOT NULL,
    score           REAL NOT NULL,
    reason          TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_findings_incident ON incident_findings(incident_id);
