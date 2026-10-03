use crate::models::{ChatSession, CodeChange, ToolType};
use crate::scanner::antigravity::unwrap_nested_str;
use crate::scanner::generic_diff::{parse_edit_change, parse_unified_diff, parse_write_change};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_claude_log(file_path: &Path, session_id: &str) -> Option<ChatSession> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut session_title = format!("Claude Session {}", &session_id[..8.min(session_id.len())]);
    let mut session_timestamp = String::new();
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
                let text_diffs = parse_unified_diff(
                    trimmed,
                    ToolType::ClaudeCode,
                    session_id,
                    &session_title,
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

        // Timestamp
        if let Some(ts) = parsed.get("timestamp").or_else(|| parsed.get("created_at")).and_then(|v| v.as_str()) {
            if session_timestamp.is_empty() {
                session_timestamp = ts.to_string();
            }
        }

        // Title from user prompt
        if let Some(msg) = parsed.get("message").or_else(|| parsed.get("content")) {
            if let Some(text) = msg.as_str() {
                if parsed.get("role").and_then(|r| r.as_str()) == Some("user") {
                    let first_line = text.lines().next().unwrap_or(text).trim();
                    if !first_line.is_empty() && first_line.len() > 3 {
                        session_title = first_line.chars().take(80).collect();
                    }
                }
            }
        }

        // Collect tool use items (top-level or inside content array)
        let mut tool_calls = Vec::new();
        if let Some(tool_name) = parsed.get("name").and_then(|n| n.as_str()) {
            let input = parsed.get("input").unwrap_or(&parsed);
            tool_calls.push((tool_name, input));
        }
        if let Some(content_arr) = parsed.get("content").and_then(|c| c.as_array()) {
            for item in content_arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(tool_name) = item.get("name").and_then(|n| n.as_str()) {
                        let input = item.get("input").unwrap_or(item);
                        tool_calls.push((tool_name, input));
                    }
                }
            }
        }

        for (tool_name, input) in tool_calls {
            let change_opt = if tool_name == "Edit" || tool_name == "Replace" {
                parse_claude_edit(session_id, &session_title, &session_timestamp, input, &mut change_counter)
            } else if tool_name == "Write" || tool_name == "write_file" {
                parse_claude_write(session_id, &session_title, &session_timestamp, input, &mut change_counter)
            } else {
                None
            };

            if let Some(change) = change_opt {
                total_added += change.lines_added;
                total_deleted += change.lines_deleted;
                changes.push(change);
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    Some(ChatSession {
        id: session_id.to_string(),
        tool: ToolType::ClaudeCode,
        title: session_title,
        timestamp: session_timestamp,
        log_path: file_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}

fn parse_claude_edit(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = input.get("file_path").or_else(|| input.get("path"))?;
    let path_str = unwrap_nested_str(file_path);
    if path_str.is_empty() {
        return None;
    }

    let old_str = input
        .get("old_string")
        .or_else(|| input.get("old_str"))
        .or_else(|| input.get("target"))
        .map(unwrap_nested_str)
        .unwrap_or_default();

    let new_str = input
        .get("new_string")
        .or_else(|| input.get("new_str"))
        .or_else(|| input.get("replacement"))
        .map(unwrap_nested_str)
        .unwrap_or_default();

    parse_edit_change(
        ToolType::ClaudeCode,
        session_id,
        session_title,
        timestamp,
        path_str,
        &old_str,
        &new_str,
        "Claude Code Edit",
        "Edit file",
        counter,
    )
}

fn parse_claude_write(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = input.get("file_path").or_else(|| input.get("path"))?;
    let path_str = unwrap_nested_str(file_path);
    if path_str.is_empty() {
        return None;
    }

    let content = input.get("content").map(unwrap_nested_str).unwrap_or_default();

    parse_write_change(
        ToolType::ClaudeCode,
        session_id,
        session_title,
        timestamp,
        path_str,
        &content,
        "Claude Code Write",
        "Write file",
        counter,
    )
}
