use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use serde::Serialize;

use crate::models::{ChatSession, FileSummary, ScanStats, ToolType};

/// Compute overall scan statistics from a list of sessions
pub fn compute_stats(sessions: &[ChatSession]) -> ScanStats {
    let mut stats = ScanStats {
        total_sessions: sessions.len(),
        ..Default::default()
    };

    let mut unique_files = HashSet::new();

    for s in sessions {
        stats.total_changes += s.changes.len();
        stats.total_lines_added += s.total_lines_added;
        stats.total_lines_deleted += s.total_lines_deleted;

        match s.tool {
            ToolType::Antigravity => stats.count_antigravity += 1,
            ToolType::ClaudeCode => stats.count_claude += 1,
            ToolType::Codex => stats.count_codex += 1,
            ToolType::GeminiCli => stats.count_gemini += 1,
            _ => {}
        }

        for c in &s.changes {
            unique_files.insert(c.file_path.clone());
        }
    }

    stats.unique_files = unique_files.len();
    stats
}

/// Aggregates and calculates file-level metrics across all sessions
/// Returns deterministic, stably sorted list (total lines changed descending, then alphabetical path)
pub fn compute_file_summaries(sessions: &[ChatSession]) -> Vec<FileSummary> {
    let mut map: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for session in sessions {
        for c in &session.changes {
            let entry = map.entry(c.file_path.clone()).or_insert((0, 0, 0));
            entry.0 += c.lines_added;
            entry.1 += c.lines_deleted;
            entry.2 += 1;
        }
    }

    let mut list: Vec<FileSummary> = map
        .into_iter()
        .map(|(file_path, (added, deleted, count))| {
            let filename = Path::new(&file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.clone());
            FileSummary {
                file_path,
                filename,
                lines_added: added,
                lines_deleted: deleted,
                change_count: count,
            }
        })
        .collect();

    // Deterministic, completely stable sort: total lines changed descending, then alphabetical path ascending
    list.sort_by(|a, b| {
        let total_a = a.lines_added + a.lines_deleted;
        let total_b = b.lines_added + b.lines_deleted;
        total_b
            .cmp(&total_a)
            .then_with(|| a.file_path.cmp(&b.file_path))
    });

    list
}

/// Filter sessions by tool type and/or text search query
pub fn filter_sessions<'a>(
    sessions: &'a [ChatSession],
    tool_filter: Option<ToolType>,
    search_query: &str,
) -> Vec<&'a ChatSession> {
    let query_lower = search_query.trim().to_lowercase();

    sessions
        .iter()
        .filter(|s| {
            if let Some(tool) = tool_filter {
                if s.tool != tool {
                    return false;
                }
            }
            s.matches_query(&query_lower)
        })
        .collect()
}

/// Find a specific session by exact ID or partial prefix match
pub fn find_session<'a>(sessions: &'a [ChatSession], id_query: &str) -> Option<&'a ChatSession> {
    let id_trimmed = id_query.trim().to_lowercase();
    if id_trimmed.is_empty() {
        return None;
    }

    // Try exact match first
    if let Some(s) = sessions.iter().find(|s| s.id.to_lowercase() == id_trimmed) {
        return Some(s);
    }

    // Fall back to prefix / partial match
    sessions
        .iter()
        .find(|s| s.id.to_lowercase().contains(&id_trimmed))
}

/// Represents a group of sessions belonging to a specific AI tool/agent
#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionGroupRef<'a> {
    pub tool: ToolType,
    pub tool_name: String,
    pub total_sessions: usize,
    pub total_lines_added: usize,
    pub total_lines_deleted: usize,
    pub sessions: Vec<&'a ChatSession>,
}

/// Group sessions by AI tool/agent, optionally filtered by a text search query
pub fn group_sessions_by_agent<'a>(
    sessions: &'a [ChatSession],
    search_query: &str,
) -> Vec<AgentSessionGroupRef<'a>> {
    let supported_tools = [
        ToolType::Antigravity,
        ToolType::ClaudeCode,
        ToolType::Codex,
        ToolType::GeminiCli,
        ToolType::Generic,
    ];

    let mut groups = Vec::new();

    for tool in supported_tools {
        let tool_sessions = filter_sessions(sessions, Some(tool), search_query);

        if !tool_sessions.is_empty() {
            let total_added: usize = tool_sessions.iter().map(|s| s.total_lines_added).sum();
            let total_deleted: usize = tool_sessions.iter().map(|s| s.total_lines_deleted).sum();
            groups.push(AgentSessionGroupRef {
                tool,
                tool_name: tool.display_name().to_string(),
                total_sessions: tool_sessions.len(),
                total_lines_added: total_added,
                total_lines_deleted: total_deleted,
                sessions: tool_sessions,
            });
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChangeType, CodeChange, ToolType};

    #[test]
    fn test_compute_stats_and_summaries() {
        let change1 = CodeChange {
            id: "c1".to_string(),
            tool: ToolType::Antigravity,
            session_id: "s1".to_string(),
            session_title: "Fix bug in main".to_string(),
            timestamp: "2026-10-01T10:00:00".to_string(),
            file_path: "src/main.rs".to_string(),
            change_type: ChangeType::Modify,
            description: "test".to_string(),
            lines_added: 15,
            lines_deleted: 5,
            diff_lines: Vec::new(),
            target_tool_action: None,
        };

        let change2 = CodeChange {
            id: "c2".to_string(),
            tool: ToolType::Antigravity,
            session_id: "s1".to_string(),
            session_title: "Fix bug in main".to_string(),
            timestamp: "2026-10-01T10:05:00".to_string(),
            file_path: "src/lib.rs".to_string(),
            change_type: ChangeType::Create,
            description: "test".to_string(),
            lines_added: 20,
            lines_deleted: 0,
            diff_lines: Vec::new(),
            target_tool_action: None,
        };

        let session = ChatSession {
            id: "s1".to_string(),
            tool: ToolType::Antigravity,
            title: "Fix bug in main".to_string(),
            timestamp: "2026-10-01T10:00:00".to_string(),
            log_path: "/tmp/log".to_string(),
            changes: vec![change1, change2],
            total_lines_added: 35,
            total_lines_deleted: 5,
        };

        let sessions = vec![session];
        let stats = compute_stats(&sessions);

        assert_eq!(stats.total_sessions, 1);
        assert_eq!(stats.total_changes, 2);
        assert_eq!(stats.total_lines_added, 35);
        assert_eq!(stats.total_lines_deleted, 5);
        assert_eq!(stats.unique_files, 2);
        assert_eq!(stats.count_antigravity, 1);

        let summaries = compute_file_summaries(&sessions);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].lines_added + summaries[0].lines_deleted, 20);

        let filtered = filter_sessions(&sessions, Some(ToolType::Antigravity), "main");
        assert_eq!(filtered.len(), 1);

        let found = find_session(&sessions, "s1");
        assert!(found.is_some());

        let groups = group_sessions_by_agent(&sessions, "");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].tool, ToolType::Antigravity);
        assert_eq!(groups[0].total_sessions, 1);
        assert_eq!(groups[0].total_lines_added, 35);
        assert_eq!(groups[0].total_lines_deleted, 5);
    }
}
