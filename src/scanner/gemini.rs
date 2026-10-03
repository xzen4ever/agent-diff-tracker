use crate::models::{ChatSession, CodeChange, ToolType};
use crate::scanner::antigravity::{parse_antigravity_transcript, unwrap_nested_str};
use crate::scanner::generic_diff::{parse_edit_change, parse_unified_diff, parse_write_change};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_gemini_log(file_path: &Path, session_id: &str) -> Option<ChatSession> {
    // If it's formatted like Antigravity transcript.jsonl, try the transcript parser
    if let Some(session) = parse_antigravity_transcript(file_path, session_id) {
        return Some(ChatSession {
            tool: ToolType::GeminiCli,
            ..session
        });
    }

    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut session_title = format!("Gemini Session {}", &session_id[..8.min(session_id.len())]);
    let mut session_timestamp = String::new();
    let mut changes = Vec::new();
    let mut change_counter = 0;
    let mut total_added = 0;
    let mut total_deleted = 0;
    let mut buffer = String::new();

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            // Check top-level session metadata (Gemini ConversationRecord header)
            if let Some(st) = v.get("startTime").and_then(|t| t.as_str()) {
                if session_timestamp.is_empty() {
                    session_timestamp = st.to_string();
                }
            }
            if let Some(sum) = v.get("summary").and_then(|s| s.as_str()) {
                let first = sum.lines().next().unwrap_or(sum).trim();
                if !first.is_empty() {
                    session_title = first.chars().take(80).collect();
                }
            }
            if let Some(ts) = v.get("timestamp").or_else(|| v.get("created_at")).and_then(|t| t.as_str()) {
                if session_timestamp.is_empty() {
                    session_timestamp = ts.to_string();
                }
            }

            // User message: prompt/content
            let msg_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if msg_type == "user" {
                if let Some(content_parts) = v.get("content").and_then(|c| c.as_array()) {
                    for part in content_parts {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            let first = text.lines().next().unwrap_or(text).trim();
                            if !first.is_empty() && first.len() > 3 && session_title.starts_with("Gemini Session") {
                                session_title = first.chars().take(80).collect();
                                break;
                            }
                        }
                    }
                }
            }

            // Gemini message: toolCalls array
            if let Some(tool_calls) = v.get("toolCalls").and_then(|tc| tc.as_array()) {
                for tc in tool_calls {
                    let tool_name = tc.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let args = tc.get("args").unwrap_or(tc);

                    let change_opt = if tool_name == "edit" {
                        parse_gemini_edit(session_id, &session_title, &session_timestamp, args, &mut change_counter)
                    } else if tool_name == "write_file" || tool_name == "writeFile" {
                        parse_gemini_write(session_id, &session_title, &session_timestamp, args, &mut change_counter)
                    } else {
                        None
                    };

                    if let Some(c) = change_opt {
                        total_added += c.lines_added;
                        total_deleted += c.lines_deleted;
                        changes.push(c);
                    }
                }
            }
        }

        buffer.push_str(trimmed);
        buffer.push('\n');
    }

    // Also parse unified diff from whole buffer if present
    let diffs = parse_unified_diff(&buffer, ToolType::GeminiCli, session_id, &session_title, &session_timestamp);
    for d in diffs {
        total_added += d.lines_added;
        total_deleted += d.lines_deleted;
        changes.push(d);
    }

    if changes.is_empty() {
        return None;
    }

    Some(ChatSession {
        id: session_id.to_string(),
        tool: ToolType::GeminiCli,
        title: session_title,
        timestamp: session_timestamp,
        log_path: file_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}

fn parse_gemini_edit(
    session_id: &str,
    session_title: &str,
    session_timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = input
        .get("file_path")
        .or_else(|| input.get("filePath"))
        .or_else(|| input.get("path"))?;
    let path_str = unwrap_nested_str(file_path);
    if path_str.is_empty() {
        return None;
    }

    let old_str = input
        .get("old_str")
        .or_else(|| input.get("old_string"))
        .map(unwrap_nested_str)
        .unwrap_or_default();

    let new_str = input
        .get("new_str")
        .or_else(|| input.get("new_string"))
        .map(unwrap_nested_str)
        .unwrap_or_default();

    parse_edit_change(
        ToolType::GeminiCli,
        session_id,
        session_title,
        session_timestamp,
        path_str,
        &old_str,
        &new_str,
        "Gemini edit",
        "Edit file",
        counter,
    )
}

fn parse_gemini_write(
    session_id: &str,
    session_title: &str,
    session_timestamp: &str,
    input: &Value,
    counter: &mut usize,
) -> Option<CodeChange> {
    let file_path = input
        .get("file_path")
        .or_else(|| input.get("filePath"))
        .or_else(|| input.get("path"))?;
    let path_str = unwrap_nested_str(file_path);
    if path_str.is_empty() {
        return None;
    }

    let content = input
        .get("content")
        .map(unwrap_nested_str)
        .unwrap_or_default();

    parse_write_change(
        ToolType::GeminiCli,
        session_id,
        session_title,
        session_timestamp,
        path_str,
        &content,
        "Gemini write_file",
        "Write file",
        counter,
    )
}
