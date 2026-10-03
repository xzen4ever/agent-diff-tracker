use crate::models::{ChatSession, ToolType};
use crate::scanner::generic_diff::parse_unified_diff;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_codex_log(file_path: &Path, session_id: &str) -> Option<ChatSession> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut session_title = format!("Codex Session {}", &session_id[..8.min(session_id.len())]);
    let mut session_timestamp = String::new();
    let mut changes = Vec::new();
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

        // Check if line contains a timestamp prefix like 2026-08-11T14:51:10
        if session_timestamp.is_empty() && trimmed.len() >= 19 && trimmed.chars().nth(4) == Some('-') {
            session_timestamp = trimmed[..19].to_string();
        }

        // Try parsing JSON RPC or structured rollout log
        if let Ok(json_val) = serde_json::from_str::<Value>(trimmed) {
            if let Some(ts) = json_val.get("timestamp").and_then(|t| t.as_str()) {
                if session_timestamp.is_empty() {
                    session_timestamp = ts.to_string();
                }
            }

            // Check for prompt or query
            if let Some(prompt) = json_val.get("prompt").or_else(|| json_val.get("query")).and_then(|p| p.as_str()) {
                let first = prompt.lines().next().unwrap_or(prompt).trim();
                if !first.is_empty() && session_title.starts_with("Codex Session") {
                    session_title = first.chars().take(80).collect();
                }
            }

            // Check for diff or patch field directly in JSON
            if let Some(patch) = json_val.get("patch").or_else(|| json_val.get("diff")).and_then(|p| p.as_str()) {
                let diffs = parse_unified_diff(patch, ToolType::Codex, session_id, &session_title, &session_timestamp);
                for d in diffs {
                    total_added += d.lines_added;
                    total_deleted += d.lines_deleted;
                    changes.push(d);
                }
            }

            // Check Codex rollout structure: {"type": "response_item", "payload": { ... }}
            if let Some(payload) = json_val.get("payload") {
                // If user message, extract session title
                if payload.get("role").and_then(|r| r.as_str()) == Some("user") {
                    if let Some(content_items) = payload.get("content").and_then(|c| c.as_array()) {
                        for item in content_items {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                let first = text.lines().next().unwrap_or(text).trim();
                                if !first.is_empty() && session_title.starts_with("Codex Session") {
                                    session_title = first.chars().take(80).collect();
                                    break;
                                }
                            }
                        }
                    }
                }

                // Check function call in payload (e.g. apply_patch)
                let func_name = payload.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args_val = payload.get("arguments");

                // arguments could be a JSON string or an object
                let patch_str = if let Some(args_str) = args_val.and_then(|a| a.as_str()) {
                    if let Ok(parsed_args) = serde_json::from_str::<Value>(args_str) {
                        parsed_args.get("patch").or_else(|| parsed_args.get("diff")).and_then(|p| p.as_str()).map(|s| s.to_string())
                    } else if func_name.contains("patch") {
                        Some(args_str.to_string())
                    } else {
                        None
                    }
                } else if let Some(args_obj) = args_val {
                    args_obj.get("patch").or_else(|| args_obj.get("diff")).and_then(|p| p.as_str()).map(|s| s.to_string())
                } else {
                    None
                };

                if let Some(patch) = patch_str {
                    let diffs = parse_unified_diff(&patch, ToolType::Codex, session_id, &session_title, &session_timestamp);
                    for d in diffs {
                        total_added += d.lines_added;
                        total_deleted += d.lines_deleted;
                        changes.push(d);
                    }
                }
            }
        }

        buffer.push_str(trimmed);
        buffer.push('\n');
    }

    // Also parse unified diff from the whole buffer if diff blocks are present
    let text_diffs = parse_unified_diff(&buffer, ToolType::Codex, session_id, &session_title, &session_timestamp);
    for d in text_diffs {
        total_added += d.lines_added;
        total_deleted += d.lines_deleted;
        changes.push(d);
    }

    if changes.is_empty() {
        return None;
    }

    Some(ChatSession {
        id: session_id.to_string(),
        tool: ToolType::Codex,
        title: session_title,
        timestamp: session_timestamp,
        log_path: file_path.to_string_lossy().to_string(),
        changes,
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
    })
}
