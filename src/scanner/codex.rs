use crate::models::{ChatSession, CodeChange, ToolType};
use crate::scanner::generic_diff::{parse_unified_diff, parse_write_change};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parses an OpenAI Codex session log (rollout JSONL or unified diff format)
pub fn parse_codex_log(file_path: &Path, default_session_id: &str) -> Option<ChatSession> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut session_id = default_session_id.to_string();
    let mut session_title = String::new();
    let mut session_timestamp = String::new();
    let mut session_cwd = String::new();
    let mut changes = Vec::new();
    let mut change_counter = 0;
    let mut total_added = 0;
    let mut total_deleted = 0;

    let mut has_json_lines = false;
    let mut raw_buffer = String::new();

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => {
                has_json_lines = true;
                v
            }
            Err(_) => {
                raw_buffer.push_str(trimmed);
                raw_buffer.push('\n');
                continue;
            }
        };

        // 1. Timestamp extraction
        if let Some(ts) = parsed
            .get("timestamp")
            .or_else(|| parsed.get("created_at"))
            .and_then(|t| t.as_str())
        {
            if session_timestamp.is_empty() && !ts.is_empty() {
                session_timestamp = ts.to_string();
            }
        }

        // 2. SessionMeta extraction (OpenAI Codex session_meta event)
        let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if event_type == "session_meta" {
            if let Some(payload) = parsed.get("payload") {
                if let Some(sid) = payload
                    .get("session_id")
                    .or_else(|| payload.get("id"))
                    .and_then(|s| s.as_str())
                {
                    if !sid.is_empty() && (session_id == default_session_id || session_id.is_empty()) {
                        session_id = sid.to_string();
                    }
                }
                if let Some(cwd) = payload.get("cwd").and_then(|c| c.as_str()) {
                    if session_cwd.is_empty() && !cwd.is_empty() {
                        session_cwd = cwd.to_string();
                    }
                }
            }
        }

        // 3. Thread/Session ID override if present at root
        if let Some(sid) = parsed
            .get("sessionId")
            .or_else(|| parsed.get("session_id"))
            .or_else(|| parsed.get("thread_id"))
            .and_then(|s| s.as_str())
        {
            if !sid.is_empty() && (session_id == default_session_id || session_id.is_empty()) {
                session_id = sid.to_string();
            }
        }

        // 4. Title / User Prompt extraction
        if session_title.is_empty() {
            if let Some(prompt) = extract_codex_prompt(&parsed) {
                let first_line = prompt.lines().next().unwrap_or(&prompt).trim();
                if !first_line.is_empty() && first_line.len() >= 3 {
                    session_title = first_line.chars().take(80).collect();
                }
            }
        }

        // 5. Code changes extraction
        let mut line_changes = Vec::new();

        // 5a. Direct apply_patch function call or patch field in payload
        if let Some(payload) = parsed.get("payload") {
            // Check for TurnDiff event inside payload
            if payload.get("type").and_then(|t| t.as_str()) == Some("turn_diff") {
                if let Some(diff) = payload.get("unified_diff").and_then(|d| d.as_str()) {
                    let diffs = parse_unified_diff(
                        diff,
                        ToolType::Codex,
                        &session_id,
                        if session_title.is_empty() { &session_id } else { &session_title },
                        &session_timestamp,
                    );
                    line_changes.extend(diffs);
                }
            }

            // Check for structured file changes map (FileChangeItem, PatchApplyEnd, PatchApplyUpdated)
            if let Some(changes_map) = payload.get("changes") {
                let parsed_map_changes = parse_codex_file_changes(
                    changes_map,
                    &session_id,
                    if session_title.is_empty() { &session_id } else { &session_title },
                    &session_timestamp,
                    &mut change_counter,
                );
                line_changes.extend(parsed_map_changes);
            }

            // Check for apply_patch tool arguments (only if not already parsed above)
            if line_changes.is_empty() {
                if let Some(patch_str) = extract_patch_from_payload(payload) {
                    let diffs = parse_unified_diff(
                        &patch_str,
                        ToolType::Codex,
                        &session_id,
                        if session_title.is_empty() { &session_id } else { &session_title },
                        &session_timestamp,
                    );
                    line_changes.extend(diffs);
                }
            }
        }

        // 5b. Direct top-level TurnDiff or FileChange
        if line_changes.is_empty() {
            if event_type == "turn_diff" {
                if let Some(diff) = parsed.get("unified_diff").and_then(|d| d.as_str()) {
                    let diffs = parse_unified_diff(
                        diff,
                        ToolType::Codex,
                        &session_id,
                        if session_title.is_empty() { &session_id } else { &session_title },
                        &session_timestamp,
                    );
                    line_changes.extend(diffs);
                }
            } else if event_type == "file_change" {
                if let Some(changes_map) = parsed.get("changes") {
                    let parsed_map_changes = parse_codex_file_changes(
                        changes_map,
                        &session_id,
                        if session_title.is_empty() { &session_id } else { &session_title },
                        &session_timestamp,
                        &mut change_counter,
                    );
                    line_changes.extend(parsed_map_changes);
                }
            }
        }

        // 5c. Direct top-level patch or diff field
        if line_changes.is_empty() {
            if let Some(patch) = parsed
                .get("patch")
                .or_else(|| parsed.get("diff"))
                .and_then(|p| p.as_str())
            {
                let diffs = parse_unified_diff(
                    patch,
                    ToolType::Codex,
                    &session_id,
                    if session_title.is_empty() { &session_id } else { &session_title },
                    &session_timestamp,
                );
                line_changes.extend(diffs);
            }
        }

        for c in line_changes {
            total_added += c.lines_added;
            total_deleted += c.lines_deleted;
            changes.push(c);
        }
    }

    // If the file was NOT structured JSONL, try parsing the entire raw buffer as a unified diff
    if !has_json_lines && !raw_buffer.trim().is_empty() {
        let text_diffs = parse_unified_diff(
            &raw_buffer,
            ToolType::Codex,
            &session_id,
            if session_title.is_empty() { &session_id } else { &session_title },
            &session_timestamp,
        );
        for d in text_diffs {
            total_added += d.lines_added;
            total_deleted += d.lines_deleted;
            changes.push(d);
        }
    }

    if changes.is_empty() {
        return None;
    }

    if session_title.is_empty() {
        let short_id = if session_id.len() > 8 {
            &session_id[..8]
        } else {
            &session_id
        };
        session_title = format!("Codex Session {}", short_id);
    }

    Some(ChatSession {
        id: session_id,
        tool: ToolType::Codex,
        title: session_title,
        timestamp: session_timestamp,
        log_path: file_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}

/// Extract user prompt / query from OpenAI Codex JSONL lines
fn extract_codex_prompt(parsed: &Value) -> Option<String> {
    // 1. Direct prompt or query field
    if let Some(prompt) = parsed
        .get("prompt")
        .or_else(|| parsed.get("query"))
        .and_then(|p| p.as_str())
    {
        return Some(prompt.to_string());
    }

    // 2. UserMessage in payload
    if let Some(payload) = parsed.get("payload") {
        let is_user = payload.get("role").and_then(|r| r.as_str()) == Some("user")
            || payload.get("type").and_then(|t| t.as_str()) == Some("user_message");

        if is_user {
            // Check content array: [{"type": "text", "text": "..."}] or UserInput::Text { text: "..." }
            if let Some(content_items) = payload.get("content").and_then(|c| c.as_array()) {
                for item in content_items {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_string());
                        }
                    }
                }
            } else if let Some(text) = payload.get("content").and_then(|c| c.as_str()) {
                return Some(text.to_string());
            } else if let Some(text) = payload.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
    }

    // 3. Root content or message
    if let Some(msg) = parsed.get("message") {
        if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
            return Some(text.to_string());
        }
    }

    None
}

/// Extracts unified diff patch string from an apply_patch tool call payload
fn extract_patch_from_payload(payload: &Value) -> Option<String> {
    let name = payload
        .get("name")
        .or_else(|| payload.get("function").and_then(|f| f.get("name")))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let args_val = payload
        .get("arguments")
        .or_else(|| payload.get("args"))
        .or_else(|| payload.get("input"))
        .or_else(|| payload.get("function").and_then(|f| f.get("arguments")));

    if let Some(args) = args_val {
        if let Some(args_str) = args.as_str() {
            // Check if arguments is a JSON string containing {"patch": "..."} or {"diff": "..."}
            if let Ok(parsed) = serde_json::from_str::<Value>(args_str) {
                if let Some(p) = parsed
                    .get("patch")
                    .or_else(|| parsed.get("diff"))
                    .or_else(|| parsed.get("unified_diff"))
                    .and_then(|s| s.as_str())
                {
                    return Some(p.to_string());
                }
            }
            // If the function name is apply_patch or contains patch, treat the string as the patch
            if name == "apply_patch" || name.contains("patch") {
                return Some(args_str.to_string());
            }
        } else if let Some(p) = args
            .get("patch")
            .or_else(|| args.get("diff"))
            .or_else(|| args.get("unified_diff"))
            .and_then(|s| s.as_str())
        {
            return Some(p.to_string());
        }
    }

    // Check direct fields
    if let Some(p) = payload
        .get("patch")
        .or_else(|| payload.get("diff"))
        .or_else(|| payload.get("unified_diff"))
        .and_then(|s| s.as_str())
    {
        return Some(p.to_string());
    }

    None
}

/// Parse OpenAI Codex HashMap<PathBuf, FileChange> structure
fn parse_codex_file_changes(
    changes_map: &Value,
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    counter: &mut usize,
) -> Vec<CodeChange> {
    let mut changes = Vec::new();
    let Some(obj) = changes_map.as_object() else {
        return changes;
    };

    for (file_path_raw, change_val) in obj {
        let file_path = file_path_raw.replace('\\', "/");
        let change_type = change_val
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        match change_type {
            "Update" | "update" => {
                if let Some(diff) = change_val
                    .get("unified_diff")
                    .or_else(|| change_val.get("diff"))
                    .and_then(|d| d.as_str())
                {
                    let diffs = parse_unified_diff(
                        diff,
                        ToolType::Codex,
                        session_id,
                        session_title,
                        timestamp,
                    );
                    changes.extend(diffs);
                }
            }
            "Add" | "add" => {
                let content = change_val
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default();
                if let Some(c) = parse_write_change(
                    ToolType::Codex,
                    session_id,
                    session_title,
                    timestamp,
                    file_path,
                    content,
                    "Codex Add File",
                    "Create file",
                    counter,
                ) {
                    changes.push(c);
                }
            }
            "Delete" | "delete" => {
                let content = change_val
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default();
                let deleted_lines = content.lines().count().max(1);
                *counter += 1;
                let mut diff_lines = Vec::new();
                diff_lines.push(crate::models::DiffLine {
                    line_type: crate::models::DiffLineType::Header,
                    old_line_num: None,
                    new_line_num: None,
                    text: format!("@@ -1,{} +0,0 @@ Codex Delete File", deleted_lines),
                });
                for (i, line) in content.lines().enumerate() {
                    diff_lines.push(crate::models::DiffLine {
                        line_type: crate::models::DiffLineType::Removed,
                        old_line_num: Some(i + 1),
                        new_line_num: None,
                        text: line.to_string(),
                    });
                }
                changes.push(CodeChange {
                    id: format!("{}_{}", session_id, counter),
                    tool: ToolType::Codex,
                    session_id: session_id.to_string(),
                    session_title: session_title.to_string(),
                    timestamp: timestamp.to_string(),
                    file_path,
                    change_type: crate::models::ChangeType::Delete,
                    description: "Codex Delete File".to_string(),
                    lines_added: 0,
                    lines_deleted: deleted_lines,
                    diff_lines,
                    target_tool_action: Some("Delete file".to_string()),
                });
            }
            _ => {
                // If it contains unified_diff directly without type
                if let Some(diff) = change_val
                    .get("unified_diff")
                    .or_else(|| change_val.get("diff"))
                    .and_then(|d| d.as_str())
                {
                    let diffs = parse_unified_diff(
                        diff,
                        ToolType::Codex,
                        session_id,
                        session_title,
                        timestamp,
                    );
                    changes.extend(diffs);
                }
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_codex_rollout_apply_patch() {
        let temp_dir = std::env::temp_dir().join(format!(
            "codex_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let log_file = temp_dir.join("rollout-2026-10-01T12-00-00-thread_123.jsonl");

        let log_content = r#"{"timestamp":"2026-10-01T12:00:00Z","type":"session_meta","payload":{"id":"thread_123","session_id":"codex-sess-1","cwd":"/repo"}}
{"timestamp":"2026-10-01T12:00:01Z","type":"response_item","payload":{"role":"user","content":[{"type":"text","text":"Implement fast sort algorithm"}]}}
{"timestamp":"2026-10-01T12:00:05Z","type":"response_item","payload":{"type":"function_call","name":"apply_patch","arguments":"{\"patch\":\"diff --git a/sort.rs b/sort.rs\\n--- a/sort.rs\\n+++ b/sort.rs\\n@@ -1,2 +1,4 @@\\n fn sort() {\\n+// fast sort\\n+let mut x = 1;\\n }\\n\"}"}}
"#;

        let mut f = File::create(&log_file).unwrap();
        f.write_all(log_content.as_bytes()).unwrap();

        let session = parse_codex_log(&log_file, "fallback_codex").expect("Should parse codex rollout");
        assert_eq!(session.id, "codex-sess-1");
        assert_eq!(session.title, "Implement fast sort algorithm");
        assert_eq!(session.total_lines_added, 2);
        assert_eq!(session.total_lines_deleted, 0);
        assert_eq!(session.changes.len(), 1);
        assert_eq!(session.changes[0].file_path, "sort.rs");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_codex_file_change_events() {
        let temp_dir = std::env::temp_dir().join(format!(
            "codex_test_events_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let log_file = temp_dir.join("rollout_events.jsonl");

        let log_content = r#"{"timestamp":"2026-10-01T12:00:00Z","type":"user_message","payload":{"type":"user_message","content":[{"type":"text","text":"Add feature and update test"}]}}
{"timestamp":"2026-10-01T12:00:05Z","type":"file_change","changes":{"src/new.rs":{"type":"Add","content":"fn hello() {}\nfn world() {}\n"}}}
{"timestamp":"2026-10-01T12:00:10Z","type":"event_msg","payload":{"type":"turn_diff","unified_diff":"--- a/test.rs\n+++ b/test.rs\n@@ -1,2 +1,2 @@\n-assert!(false);\n+assert!(true);\n"}}
"#;

        let mut f = File::create(&log_file).unwrap();
        f.write_all(log_content.as_bytes()).unwrap();

        let session = parse_codex_log(&log_file, "sess_events").expect("Should parse events");
        assert_eq!(session.title, "Add feature and update test");
        assert_eq!(session.changes.len(), 2);
        assert_eq!(session.total_lines_added, 2 + 1); // 2 added in new.rs, 1 added in test.rs
        assert_eq!(session.total_lines_deleted, 1);    // 1 deleted in test.rs

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
