use crate::models::{ChangeType, ChatSession, CodeChange, DiffLine, DiffLineType, ToolType};
use crate::scanner::generic_diff::parse_write_change;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Helper to decode values that might be double-serialized JSON strings or regular strings
pub fn unwrap_nested_str(val: &Value) -> String {
    match val {
        Value::String(s) => {
            let trimmed = s.trim();
            if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
            {
                if let Ok(inner) = serde_json::from_str::<String>(trimmed) {
                    return inner;
                }
            }
            if s.contains("\\u") || s.contains("\\n") {
                let wrapped = format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
                if let Ok(decoded) = serde_json::from_str::<String>(&wrapped) {
                    return decoded;
                }
            }
            s.clone()
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => val.to_string(),
    }
}

pub fn parse_antigravity_transcript(transcript_path: &Path, session_id: &str) -> Option<ChatSession> {
    let file = File::open(transcript_path).ok()?;
    let reader = BufReader::new(file);

    let mut session_title = String::new();
    let mut session_timestamp = String::new();
    let mut changes = Vec::new();
    let mut total_added = 0;
    let mut total_deleted = 0;
    let mut change_counter = 0;

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
            Err(_) => continue,
        };

        // Extract session title and timestamp from the first USER_INPUT
        if session_title.is_empty() {
            if let Some(step_type) = parsed.get("type").and_then(|t| t.as_str()) {
                if step_type == "USER_INPUT" {
                    if let Some(ts) = parsed.get("created_at").and_then(|t| t.as_str()) {
                        session_timestamp = ts.to_string();
                    }
                    if let Some(content) = parsed.get("content").and_then(|c| c.as_str()) {
                        session_title = extract_user_request(content);
                    }
                }
            }
        }

        // Also fallback timestamp if still empty
        if session_timestamp.is_empty() {
            if let Some(ts) = parsed.get("created_at").and_then(|t| t.as_str()) {
                session_timestamp = ts.to_string();
            }
        }

        // Check for tool_calls in the step
        if let Some(tool_calls) = parsed.get("tool_calls").and_then(|tc| tc.as_array()) {
            for call in tool_calls {
                let tool_name = match call.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n,
                    None => continue,
                };

                let args = match call.get("args") {
                    Some(a) => a,
                    None => continue,
                };

                let step_created_at = parsed
                    .get("created_at")
                    .and_then(|t| t.as_str())
                    .unwrap_or(&session_timestamp);

                if tool_name == "write_to_file" {
                    if let Some(change) = parse_write_to_file(
                        session_id,
                        &session_title,
                        step_created_at,
                        args,
                        &mut change_counter,
                    ) {
                        total_added += change.lines_added;
                        total_deleted += change.lines_deleted;
                        changes.push(change);
                    }
                } else if tool_name == "replace_file_content" {
                    if let Some(change) = parse_replace_file_content(
                        session_id,
                        &session_title,
                        step_created_at,
                        args,
                        &mut change_counter,
                    ) {
                        total_added += change.lines_added;
                        total_deleted += change.lines_deleted;
                        changes.push(change);
                    }
                }
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    if session_title.is_empty() {
        session_title = format!("Session {}", &session_id[..8.min(session_id.len())]);
    }

    Some(ChatSession {
        id: session_id.to_string(),
        tool: ToolType::Antigravity,
        title: session_title,
        timestamp: session_timestamp,
        log_path: transcript_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}

fn extract_user_request(raw_content: &str) -> String {
    if let Some(start_idx) = raw_content.find("<USER_REQUEST>") {
        let content_after = &raw_content[start_idx + "<USER_REQUEST>".len()..];
        if let Some(end_idx) = content_after.find("</USER_REQUEST>") {
            let extracted = content_after[..end_idx].trim();
            let first_line = extracted.lines().next().unwrap_or(extracted).trim();
            if !first_line.is_empty() {
                return truncate_str(first_line, 80);
            }
        }
    }

    let first_line = raw_content.lines().next().unwrap_or(raw_content).trim();
    truncate_str(first_line, 80)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        format!("{}...", s.chars().take(max_len).collect::<String>())
    } else {
        s.to_string()
    }
}

fn parse_write_to_file(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    args: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let target_file_raw = args.get("TargetFile")?;
    let target_file = unwrap_nested_str(target_file_raw);
    if target_file.is_empty() {
        return None;
    }

    let code_content_raw = args.get("CodeContent")?;
    let code_content = unwrap_nested_str(code_content_raw);

    let desc = args
        .get("Description")
        .map(unwrap_nested_str)
        .unwrap_or_else(|| "Create/Overwrite file".to_string());

    let tool_action = args
        .get("toolAction")
        .map(unwrap_nested_str)
        .unwrap_or_else(|| "write_to_file".to_string());

    parse_write_change(
        ToolType::Antigravity,
        session_id,
        session_title,
        timestamp,
        target_file,
        &code_content,
        &desc,
        &tool_action,
        counter,
    )
}

fn parse_replace_file_content(
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    args: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let target_file_raw = args.get("TargetFile")?;
    let target_file = unwrap_nested_str(target_file_raw);
    if target_file.is_empty() {
        return None;
    }

    let target_content = args
        .get("TargetContent")
        .map(unwrap_nested_str)
        .unwrap_or_default();
    let replacement_content = args
        .get("ReplacementContent")
        .map(unwrap_nested_str)
        .unwrap_or_default();

    let start_line: usize = args
        .get("StartLine")
        .map(unwrap_nested_str)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);

    let desc = args
        .get("Description")
        .map(unwrap_nested_str)
        .or_else(|| args.get("Instruction").map(unwrap_nested_str))
        .unwrap_or_else(|| "Replace file content".to_string());

    let tool_action = args.get("toolAction").map(unwrap_nested_str);

    let old_lines: Vec<&str> = if target_content.is_empty() {
        Vec::new()
    } else {
        target_content.lines().collect()
    };

    let new_lines: Vec<&str> = if replacement_content.is_empty() {
        Vec::new()
    } else {
        replacement_content.lines().collect()
    };

    let lines_deleted = old_lines.len();
    let lines_added = new_lines.len();

    if lines_deleted == 0 && lines_added == 0 {
        return None;
    }

    *counter += 1;
    let mut diff_lines = Vec::new();

    diff_lines.push(DiffLine {
        line_type: DiffLineType::Header,
        old_line_num: None,
        new_line_num: None,
        text: format!(
            "@@ -{},{} +{},{} @@ {}",
            start_line, lines_deleted, start_line, lines_added, desc
        ),
    });

    for (i, line) in old_lines.into_iter().enumerate() {
        diff_lines.push(DiffLine {
            line_type: DiffLineType::Removed,
            old_line_num: Some(start_line + i),
            new_line_num: None,
            text: line.to_string(),
        });
    }

    for (i, line) in new_lines.into_iter().enumerate() {
        diff_lines.push(DiffLine {
            line_type: DiffLineType::Added,
            old_line_num: None,
            new_line_num: Some(start_line + i),
            text: line.to_string(),
        });
    }

    Some(CodeChange {
        id: format!("{}_r_{}", session_id, counter),
        tool: ToolType::Antigravity,
        session_id: session_id.to_string(),
        session_title: session_title.to_string(),
        timestamp: timestamp.to_string(),
        file_path: target_file,
        change_type: ChangeType::Modify,
        description: desc,
        lines_added,
        lines_deleted,
        diff_lines,
        target_tool_action: tool_action,
    })
}
