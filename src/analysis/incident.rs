use anyhow::{Context, Result};
use crate::graph::neo4j::Neo4jGraph;
use crate::models::{IncidentCandidate, IncidentResult};

use super::impact::parse_node_ref;

/// Critical service names that get bonus scoring.
const CRITICAL_SERVICES: &[&str] = &[
    "postgresql", "postgres", "postmaster", "nginx", "docker",
    "containerd", "redis", "mysql", "sshd", "haproxy",
];

/// Analyze an incident by finding upstream candidates and scoring them.
///
/// Heuristic scoring:
/// - Base score for being upstream: 0.3
/// - Closer proximity (lower depth): +0.15 for depth=1, +0.10 for depth=2, etc.
/// - Failed/unhealthy status: +0.15
/// - More downstream dependents: +0.10
/// - Critical service bonus: +0.10
/// - Higher confidence on dependency edge: multiplier
pub async fn analyze(neo: &Neo4jGraph, symptom: &str, affected: &str) -> Result<IncidentResult> {
    let (affected_type, affected_id) = parse_node_ref(affected);

    // Get upstream candidates
    let raw_candidates = neo
        .get_upstream_candidates(affected_type, affected_id)
        .await
        .context("Failed to query upstream candidates from Neo4j")?;

    let mut candidates: Vec<IncidentCandidate> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (cand_type, cand_id, status, depth, confidence) in &raw_candidates {
        let node_key = format!("{}:{}", cand_type, cand_id);

        // Deduplicate
        if !seen.insert(node_key.clone()) {
            continue;
        }

        // Base score for being upstream
        let mut score = 0.3;
        let mut reasons = Vec::new();

        reasons.push(format!("upstream node of {}", affected));

        // Proximity bonus (closer = higher score)
        let proximity_bonus = match *depth {
            1 => 0.20,
            2 => 0.12,
            3 => 0.08,
            _ => 0.03,
        };
        score += proximity_bonus;
        if *depth <= 2 {
            reasons.push(format!("direct dependency (depth={})", depth));
        }

        // Failed/unhealthy status bonus
        let status_lower = status.to_lowercase();
        if status_lower.contains("failed")
            || status_lower.contains("dead")
            || status_lower.contains("unhealthy")
            || status_lower.contains("exited")
        {
            score += 0.15;
            reasons.push(format!("status is '{}'", status));
        }

        // Downstream dependents bonus
        let downstream_count = neo
            .count_downstream(cand_type, cand_id)
            .await
            .unwrap_or(0);
        if downstream_count > 2 {
            score += 0.10;
            reasons.push(format!("{} downstream dependents", downstream_count));
        }

        // Critical service bonus
        let cand_lower = cand_id.to_lowercase();
        if CRITICAL_SERVICES.iter().any(|&s| cand_lower.contains(s)) {
            score += 0.10;
            reasons.push("critical infrastructure service".to_string());
        }

        // Confidence multiplier
        score *= confidence.max(0.5);

        // Get path from candidate to affected
        let path = neo
            .get_path_to_affected(cand_type, cand_id, affected_type, affected_id)
            .await
            .unwrap_or_default();

        // Build readable path, or use a simple one
        let display_path = if path.is_empty() {
            vec![
                format!("{}:{}", affected_type, affected_id),
                format!("{}:{}", cand_type, cand_id),
            ]
        } else {
            // Reverse so it reads from affected to candidate
            let mut p = path;
            p.reverse();
            p
        };

        candidates.push(IncidentCandidate {
            node: node_key,
            score: (score * 100.0).round() / 100.0, // Round to 2 decimal places
            reason: reasons.join("; "),
            path: display_path,
        });
    }

    // Sort by score descending, take top 5
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(5);

    Ok(IncidentResult {
        symptom: symptom.to_string(),
        affected: affected.to_string(),
        candidates,
    })
}
