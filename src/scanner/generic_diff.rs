use crate::models::{ChangeType, CodeChange, DiffLine, DiffLineType, ToolType};

/// Parses unified diff text into a list of CodeChange structs.
pub fn parse_unified_diff(
    text: &str,
    tool: ToolType,
    session_id: &str,
    session_title: &str,
    timestamp: &str,
) -> Vec<CodeChange> {
    let mut changes = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_lines = Vec::new();
    let mut lines_added = 0;
    let mut lines_deleted = 0;
    let mut old_num = 0;
    let mut new_num = 0;
    let mut change_counter = 0;

    let flush_change = |changes: &mut Vec<CodeChange>,
                        file: &mut Option<String>,
                        lines: &mut Vec<DiffLine>,
                        added: &mut usize,
                        deleted: &mut usize,
                        counter: &mut usize| {
        if let Some(file_path) = file.take() {
            if !lines.is_empty() {
                *counter += 1;
                let c_type = if *deleted == 0 && *added > 0 {
                    ChangeType::Create
                } else if *added == 0 && *deleted > 0 {
                    ChangeType::Delete
                } else {
                    ChangeType::Modify
                };

                changes.push(CodeChange {
                    id: format!("{}_{}", session_id, counter),
                    tool,
                    session_id: session_id.to_string(),
                    session_title: session_title.to_string(),
                    timestamp: timestamp.to_string(),
                    file_path,
                    change_type: c_type,
                    description: "Diff patch".to_string(),
                    lines_added: *added,
                    lines_deleted: *deleted,
                    diff_lines: std::mem::take(lines),
                    target_tool_action: Some("Apply diff patch".to_string()),
                });
                *added = 0;
                *deleted = 0;
            }
        }
    };

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if line.starts_with("diff --git ") {
            flush_change(
                &mut changes,
                &mut current_file,
                &mut current_lines,
                &mut lines_added,
                &mut lines_deleted,
                &mut change_counter,
            );
            // Parse b/path
            if let Some(pos) = line.rfind(" b/") {
                current_file = Some(line[pos + 3..].to_string());
            }
        } else if let Some(stripped) = line.strip_prefix("--- ") {
            if current_file.is_none() {
                let path = stripped.trim();
                let clean_path = path.strip_prefix("a/").unwrap_or(path);
                if clean_path != "/dev/null" {
                    current_file = Some(clean_path.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix("+++ ") {
            let path = stripped.trim();
            let clean_path = path.strip_prefix("b/").unwrap_or(path);
            if clean_path != "/dev/null" {
                current_file = Some(clean_path.to_string());
            }
        } else if line.starts_with("@@") {
            // Header: @@ -1,5 +1,6 @@
            current_lines.push(DiffLine {
                line_type: DiffLineType::Header,
                old_line_num: None,
                new_line_num: None,
                text: line.to_string(),
            });
            // Try to extract starting line numbers
            if let Some(caps) = parse_hunk_header(line) {
                old_num = caps.0;
                new_num = caps.1;
            }
        } else if let Some(content) = line.strip_prefix('+') {
            lines_added += 1;
            new_num += 1;
            current_lines.push(DiffLine {
                line_type: DiffLineType::Added,
                old_line_num: None,
                new_line_num: Some(new_num),
                text: content.to_string(),
            });
        } else if let Some(content) = line.strip_prefix('-') {
            lines_deleted += 1;
            old_num += 1;
            current_lines.push(DiffLine {
                line_type: DiffLineType::Removed,
                old_line_num: Some(old_num),
                new_line_num: None,
                text: content.to_string(),
            });
        } else if let Some(content) = line.strip_prefix(' ') {
            old_num += 1;
            new_num += 1;
            current_lines.push(DiffLine {
                line_type: DiffLineType::Context,
                old_line_num: Some(old_num),
                new_line_num: Some(new_num),
                text: content.to_string(),
            });
        }
    }

    flush_change(
        &mut changes,
        &mut current_file,
        &mut current_lines,
        &mut lines_added,
        &mut lines_deleted,
        &mut change_counter,
    );

    changes
}

fn parse_hunk_header(header: &str) -> Option<(usize, usize)> {
    // Format: @@ -old_start,old_count +new_start,new_count @@
    let parts: Vec<&str> = header.split("@@").collect();
    if parts.len() < 2 {
        return None;
    }
    let middle = parts[1].trim();
    let tokens: Vec<&str> = middle.split_whitespace().collect();
    let mut old_start: usize = 1;
    let mut new_start: usize = 1;

    for tok in tokens {
        if let Some(val) = tok.strip_prefix('-') {
            if let Some(first) = val.split(',').next() {
                old_start = first.parse().unwrap_or(1);
            }
        } else if let Some(val) = tok.strip_prefix('+') {
            if let Some(first) = val.split(',').next() {
                new_start = first.parse().unwrap_or(1);
            }
        }
    }

    Some((old_start.saturating_sub(1), new_start.saturating_sub(1)))
}

/// Helper to construct a CodeChange for an Edit/Replace action
#[allow(clippy::too_many_arguments)]
pub fn parse_edit_change(
    tool: ToolType,
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    file_path: String,
    old_str: &str,
    new_str: &str,
    desc: &str,
    tool_action: &str,
    counter: &mut usize,
) -> Option<CodeChange> {
    let old_lines: Vec<&str> = if old_str.is_empty() { Vec::new() } else { old_str.lines().collect() };
    let new_lines: Vec<&str> = if new_str.is_empty() { Vec::new() } else { new_str.lines().collect() };

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
        text: format!("@@ -1,{} +1,{} @@ {}", lines_deleted, lines_added, desc),
    });

    for (i, line) in old_lines.into_iter().enumerate() {
        diff_lines.push(DiffLine {
            line_type: DiffLineType::Removed,
            old_line_num: Some(i + 1),
            new_line_num: None,
            text: line.to_string(),
        });
    }

    for (i, line) in new_lines.into_iter().enumerate() {
        diff_lines.push(DiffLine {
            line_type: DiffLineType::Added,
            old_line_num: None,
            new_line_num: Some(i + 1),
            text: line.to_string(),
        });
    }

    let prefix = match tool {
        ToolType::ClaudeCode => "claude_e",
        ToolType::GeminiCli => "gemini_e",
        _ => "e",
    };

    Some(CodeChange {
        id: format!("{}_{}_{}", session_id, prefix, counter),
        tool,
        session_id: session_id.to_string(),
        session_title: session_title.to_string(),
        timestamp: timestamp.to_string(),
        file_path,
        change_type: ChangeType::Modify,
        description: desc.to_string(),
        lines_added,
        lines_deleted,
        diff_lines,
        target_tool_action: Some(tool_action.to_string()),
    })
}

/// Helper to construct a CodeChange for a Write/Create action
#[allow(clippy::too_many_arguments)]
pub fn parse_write_change(
    tool: ToolType,
    session_id: &str,
    session_title: &str,
    timestamp: &str,
    file_path: String,
    content: &str,
    desc: &str,
    tool_action: &str,
    counter: &mut usize,
) -> Option<CodeChange> {
    let content_lines: Vec<&str> = content.lines().collect();
    let lines_count = content_lines.len();

    *counter += 1;
    let mut diff_lines = Vec::new();
    diff_lines.push(DiffLine {
        line_type: DiffLineType::Header,
        old_line_num: None,
        new_line_num: None,
        text: format!("@@ +1,{} @@ {}", lines_count, desc),
    });

    for (i, line) in content_lines.into_iter().enumerate() {
        diff_lines.push(DiffLine {
            line_type: DiffLineType::Added,
            old_line_num: None,
            new_line_num: Some(i + 1),
            text: line.to_string(),
        });
    }

    let prefix = match tool {
        ToolType::ClaudeCode => "claude_w",
        ToolType::GeminiCli => "gemini_w",
        _ => "w",
    };

    Some(CodeChange {
        id: format!("{}_{}_{}", session_id, prefix, counter),
        tool,
        session_id: session_id.to_string(),
        session_title: session_title.to_string(),
        timestamp: timestamp.to_string(),
        file_path,
        change_type: ChangeType::Create,
        description: desc.to_string(),
        lines_added: lines_count,
        lines_deleted: 0,
        diff_lines,
        target_tool_action: Some(tool_action.to_string()),
    })
}
