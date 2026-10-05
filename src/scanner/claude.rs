use crate::models::{ChatSession, CodeChange, ToolType};
use crate::scanner::antigravity::unwrap_nested_str;
use crate::scanner::generic_diff::{parse_edit_change, parse_unified_diff, parse_write_change};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parses a Claude Code session log file (JSONL or unified diff format)
pub fn parse_claude_log(file_path: &Path, default_session_id: &str) -> Option<ChatSession> {
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
            Ok(v) => v,
            Err(_) => {
                // If it's not valid JSON, check if it's raw unified diff
                let text_diffs = parse_unified_diff(
                    trimmed,
                    ToolType::ClaudeCode,
                    &session_id,
                    if session_title.is_empty() { &session_id } else { &session_title },
                    &session_timestamp,
                );
                for d in text_diffs {
                    total_added += d.lines_added;
                    total_deleted += d.lines_deleted;
                    changes.push(d);
                }
                continue;
            }
        };

        // 1. Session ID override if present in JSONL event
        if let Some(sid) = parsed
            .get("sessionId")
            .or_else(|| parsed.get("session_id"))
            .and_then(|v| v.as_str())
        {
            if !sid.is_empty() && (session_id == default_session_id || session_id.is_empty()) {
                session_id = sid.to_string();
            }
        }

        // 2. Working directory (cwd)
        if let Some(cwd) = parsed.get("cwd").and_then(|v| v.as_str()) {
            if session_cwd.is_empty() && !cwd.is_empty() {
                session_cwd = cwd.to_string();
            }
        }

        // 3. Timestamp
        if let Some(ts) = parsed
            .get("timestamp")
            .or_else(|| parsed.get("created_at"))
            .or_else(|| parsed.get("time"))
            .and_then(|v| v.as_str())
        {
            if session_timestamp.is_empty() && !ts.is_empty() {
                session_timestamp = ts.to_string();
            }
        }

        // 4. Title / User Prompt extraction
        if session_title.is_empty() {
            if let Some(prompt) = extract_user_prompt(&parsed) {
                let first_line = prompt.lines().next().unwrap_or(&prompt).trim();
                let clean = first_line.trim_start_matches("/ask ").trim_start_matches("/chat ");
                if !clean.is_empty() && clean.len() >= 3 {
                    session_title = clean.chars().take(80).collect();
                }
            }
        }

        // 5. Collect all tool calls in this line (Claude Code nested & flat schemas)
        let tool_calls = extract_all_tool_calls(&parsed);

        for (tool_name, input) in tool_calls {
            let tool_lower = tool_name.to_lowercase();
            let parsed_changes = parse_claude_tool_call(
                &tool_lower,
                &session_id,
                if session_title.is_empty() { &session_id } else { &session_title },
                &session_timestamp,
                &session_cwd,
                input,
                &mut change_counter,
            );

            for change in parsed_changes {
                total_added += change.lines_added;
                total_deleted += change.lines_deleted;
                changes.push(change);
            }
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
        session_title = format!("Claude Session {}", short_id);
    }

    Some(ChatSession {
        id: session_id,
        tool: ToolType::ClaudeCode,
        title: session_title,
        timestamp: session_timestamp,
        log_path: file_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}

/// Extracts user prompt from multiple Claude Code message formats
fn extract_user_prompt(parsed: &Value) -> Option<String> {
    let msg_type = parsed.get("type").and_then(|v| v.as_str());
    let role = parsed
        .get("role")
        .or_else(|| parsed.get("message").and_then(|m| m.get("role")))
        .and_then(|v| v.as_str());

    let is_user = msg_type == Some("user") || role == Some("user");
    if !is_user {
        return None;
    }

    // Check parsed.message (object or string)
    if let Some(msg) = parsed.get("message") {
        if let Some(s) = msg.as_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if let Some(content) = msg.get("content") {
            if let Some(text) = extract_text_from_content(content) {
                return Some(text);
            }
        }
    }

    // Check parsed.content (string or array)
    if let Some(content) = parsed.get("content") {
        if let Some(text) = extract_text_from_content(content) {
            return Some(text);
        }
    }

    // Check parsed.text or parsed.prompt
    if let Some(t) = parsed
        .get("text")
        .or_else(|| parsed.get("prompt"))
        .and_then(|v| v.as_str())
    {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Helper to decode text content that can be a plain string or an array of blocks
fn extract_text_from_content(content: &Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(arr) = content.as_array() {
        for item in arr {
            let item_type = item.get("type").and_then(|v| v.as_str());
            if item_type == Some("text") || item_type.is_none() {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    let trimmed = t.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Collect all tool call (name, input) pairs across various Claude Code formats
fn extract_all_tool_calls<'a>(parsed: &'a Value) -> Vec<(&'a str, &'a Value)> {
    let mut list = Vec::new();

    // 1. Check parsed["message"]["content"] array (standard Claude Code CLI JSONL)
    if let Some(msg) = parsed.get("message") {
        if let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
            for item in content_arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        let input = item.get("input").unwrap_or(item);
                        list.push((name, input));
                    }
                }
            }
        }
    }

    // 2. Check parsed["content"] array directly
    if let Some(content_arr) = parsed.get("content").and_then(|c| c.as_array()) {
        for item in content_arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    let input = item.get("input").unwrap_or(item);
                    list.push((name, input));
                }
            }
        }
    }

    // 3. Top-level tool_use entry
    if parsed.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
        if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
            let input = parsed.get("input").unwrap_or(parsed);
            list.push((name, input));
        }
    }

    // 4. Direct top-level name and input
    if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
        if list.is_empty() {
            let input = parsed.get("input").unwrap_or(parsed);
            list.push((name, input));
        }
    }

    // 5. Tool calls in OpenAI/Anthropic "tool_calls" array
    if let Some(tool_calls) = parsed.get("tool_calls").and_then(|tc| tc.as_array()) {
        for tc in tool_calls {
            let name = tc
                .get("name")
                .or_else(|| tc.get("function").and_then(|f| f.get("name")))
                .and_then(|n| n.as_str());
            let input = tc
                .get("input")
                .or_else(|| tc.get("args"))
                .or_else(|| tc.get("function").and_then(|f| f.get("arguments")))
                .unwrap_or(tc);
            if let Some(n) = name {
                list.push((n, input));
            }
        }
    }

    list
}

/// Dispatches a tool call to the appropriate file diff generator
fn parse_claude_tool_call(
    tool_lower: &str,
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    _session_cwd: &str,
    input: &Value,
    counter: &mut usize,
) -> Vec<CodeChange> {
    let mut changes = Vec::new();

    let is_edit = tool_lower == "edit"
        || tool_lower == "fileedittool"
        || tool_lower == "str_replace_editor"
        || tool_lower == "str_replace"
        || tool_lower == "replace"
        || tool_lower == "multiedit"
        || tool_lower == "editor";

    let is_write = tool_lower == "write"
        || tool_lower == "filewritetool"
        || tool_lower == "write_file"
        || tool_lower == "create_file"
        || tool_lower == "create";

    let is_notebook = tool_lower == "notebookedit" || tool_lower == "notebookeditcell";
    let is_patch = tool_lower == "patch" || tool_lower == "apply_patch";

    if is_edit {
        // str_replace_editor with command == "create" is actually a file creation
        if let Some(cmd) = input.get("command").and_then(|c| c.as_str()) {
            if cmd == "create" {
                if let Some(change) = parse_claude_write_item(session_id, session_title, timestamp, input, counter) {
                    changes.push(change);
                }
                return changes;
            }
        }

        // Check for MultiEdit (array of edits)
        if let Some(edits) = input.get("edits").and_then(|e| e.as_array()) {
            if let Some(file_path) = extract_file_path(input) {
                for edit in edits {
                    let old_str = extract_old_string(edit);
                    let new_str = extract_new_string(edit);
                    if let Some(change) = parse_edit_change(
                        ToolType::ClaudeCode,
                        session_id,
                        session_title,
                        timestamp,
                        file_path.clone(),
                        &old_str,
                        &new_str,
                        "Claude Code MultiEdit",
                        "MultiEdit file",
                        counter,
                    ) {
                        changes.push(change);
                    }
                }
                return changes;
            }
        }

        // Single Edit
        if let Some(change) = parse_claude_edit_item(session_id, session_title, timestamp, input, counter) {
            changes.push(change);
        }
    } else if is_write {
        if let Some(change) = parse_claude_write_item(session_id, session_title, timestamp, input, counter) {
            changes.push(change);
        }
    } else if is_notebook {
        if let Some(file_path) = extract_file_path(input) {
            let content = input
                .get("new_source")
                .or_else(|| input.get("content"))
                .or_else(|| input.get("cell_content"))
                .map(unwrap_nested_str)
                .unwrap_or_default();
            if !content.is_empty() {
                if let Some(change) = parse_write_change(
                    ToolType::ClaudeCode,
                    session_id,
                    session_title,
                    timestamp,
                    file_path,
                    &content,
                    "Claude Notebook Edit",
                    "Edit notebook cell",
                    counter,
                ) {
                    changes.push(change);
                }
            }
        }
    } else if is_patch {
        if let Some(patch_str) = input.get("patch").or_else(|| input.get("diff")).and_then(|v| v.as_str()) {
            let text_diffs = parse_unified_diff(
                patch_str,
                ToolType::ClaudeCode,
                session_id,
                session_title,
                timestamp,
            );
            changes.extend(text_diffs);
        }
    }

    changes
}

fn extract_file_path(input: &Value) -> Option<String> {
    let raw = input
        .get("file_path")
        .or_else(|| input.get("path"))
        .or_else(|| input.get("target_file"))
        .or_else(|| input.get("notebook_path"))?;
    let path_str = unwrap_nested_str(raw);
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.replace('\\', "/"))
}

fn extract_old_string(input: &Value) -> String {
    input
        .get("old_string")
        .or_else(|| input.get("old_str"))
        .or_else(|| input.get("target"))
        .or_else(|| input.get("find"))
        .map(unwrap_nested_str)
        .unwrap_or_default()
}

fn extract_new_string(input: &Value) -> String {
    input
        .get("new_string")
        .or_else(|| input.get("new_str"))
        .or_else(|| input.get("replacement"))
        .or_else(|| input.get("replace"))
        .map(unwrap_nested_str)
        .unwrap_or_default()
}

fn parse_claude_edit_item(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = extract_file_path(input)?;
    let old_str = extract_old_string(input);
    let new_str = extract_new_string(input);

    parse_edit_change(
        ToolType::ClaudeCode,
        session_id,
        session_title,
        timestamp,
        file_path,
        &old_str,
        &new_str,
        "Claude Code Edit",
        "Edit file",
        counter,
    )
}

fn parse_claude_write_item(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = extract_file_path(input)?;
    let content = input
        .get("content")
        .or_else(|| input.get("file_text"))
        .or_else(|| input.get("text"))
        .map(unwrap_nested_str)
        .unwrap_or_default();

    parse_write_change(
        ToolType::ClaudeCode,
        session_id,
        session_title,
        timestamp,
        file_path,
        &content,
        "Claude Code Write",
        "Write file",
        counter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_claude_standard_jsonl() {
        let temp_dir = std::env::temp_dir().join(format!(
            "claude_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let log_file = temp_dir.join("test_session.jsonl");

        let log_content = r#"{"type":"user","message":{"role":"user","content":"Create a rust utility function"},"timestamp":"2026-10-01T10:00:00Z","sessionId":"sess_claude_100"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I will create the util file."},{"type":"tool_use","id":"tu_1","name":"Write","input":{"file_path":"src/utils.rs","content":"pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n"}}]},"timestamp":"2026-10-01T10:00:05Z"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu_2","name":"Edit","input":{"file_path":"src/utils.rs","old_string":"    a + b\n","new_string":"    // add numbers\n    a + b\n"}}]},"timestamp":"2026-10-01T10:00:10Z"}
"#;

        let mut f = File::create(&log_file).unwrap();
        f.write_all(log_content.as_bytes()).unwrap();

        let session = parse_claude_log(&log_file, "fallback_id").expect("Should parse session successfully");
        assert_eq!(session.id, "sess_claude_100");
        assert_eq!(session.title, "Create a rust utility function");
        assert_eq!(session.timestamp, "2026-10-01T10:00:00Z");
        assert_eq!(session.changes.len(), 2);
        assert_eq!(session.total_lines_added, 3 + 2); // 3 lines written + 2 lines added in edit
        assert_eq!(session.total_lines_deleted, 1);    // 1 line deleted in edit

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_claude_str_replace_and_multiedit() {
        let temp_dir = std::env::temp_dir().join(format!(
            "claude_test_multi_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let log_file = temp_dir.join("test_session_multi.jsonl");

        let log_content = r#"{"type":"user","content":[{"type":"text","text":"Refactor config"}]}
{"type":"tool_use","name":"str_replace_editor","input":{"command":"str_replace","path":"config.toml","old_str":"debug = false","new_str":"debug = true\nverbose = true"}}
{"type":"tool_use","name":"MultiEdit","input":{"path":"config.toml","edits":[{"old_string":"port = 80","new_string":"port = 8080"},{"old_string":"host = localhost","new_string":"host = 0.0.0.0"}]}}
"#;

        let mut f = File::create(&log_file).unwrap();
        f.write_all(log_content.as_bytes()).unwrap();

        let session = parse_claude_log(&log_file, "sess_multi").expect("Should parse session");
        assert_eq!(session.title, "Refactor config");
        assert_eq!(session.changes.len(), 3); // 1 str_replace + 2 from MultiEdit
        assert_eq!(session.total_lines_added, 2 + 1 + 1);
        assert_eq!(session.total_lines_deleted, 1 + 1 + 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
