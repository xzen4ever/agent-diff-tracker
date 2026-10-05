use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    Antigravity,
    ClaudeCode,
    Codex,
    GeminiCli,
    Generic,
}

impl ToolType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ToolType::Antigravity => "Antigravity",
            ToolType::ClaudeCode => "Claude Code",
            ToolType::Codex => "OpenAI Codex",
            ToolType::GeminiCli => "Gemini CLI",
            ToolType::Generic => "Other / Diff Log",
        }
    }

    pub fn badge_short(&self) -> &'static str {
        match self {
            ToolType::Antigravity => "AGY",
            ToolType::ClaudeCode => "CLAUDE",
            ToolType::Codex => "CODEX",
            ToolType::GeminiCli => "GEMINI",
            ToolType::Generic => "DIFF",
        }
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        match self {
            ToolType::Antigravity => [79, 140, 255, 255],  // Google Blue
            ToolType::ClaudeCode => [217, 119, 87, 255],   // Anthropic Terracotta
            ToolType::Codex => [56, 189, 248, 255],       // Cyan (OpenAI Codex)
            ToolType::GeminiCli => [168, 85, 247, 255],    // Purple / Gemini
            ToolType::Generic => [156, 163, 175, 255],    // Gray
        }
    }

    pub fn color_for_mode(&self, is_light: bool) -> [u8; 4] {
        if is_light {
            match self {
                ToolType::Antigravity => [26, 115, 232, 255],
                ToolType::ClaudeCode => [194, 65, 12, 255],
                ToolType::Codex => [2, 132, 199, 255],
                ToolType::GeminiCli => [126, 34, 206, 255],
                ToolType::Generic => [75, 85, 99, 255],
            }
        } else {
            self.color_rgba()
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ToolType::Antigravity => "Google DeepMind's Advanced Agentic Coding CLI & Assistant",
            ToolType::ClaudeCode => "Anthropic's Agentic Terminal Coding & Execution Tool",
            ToolType::Codex => "OpenAI Codex CLI Sessions & Rollouts",
            ToolType::GeminiCli => "Google Gemini CLI Developer Coding Assistant",
            ToolType::Generic => "Unified Git & Standard Patch Diff Logs",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ToolType::Antigravity => "◈",
            ToolType::ClaudeCode => "⌥",
            ToolType::Codex => "⌘",
            ToolType::GeminiCli => "❖",
            ToolType::Generic => "≡",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
}

impl ChangeType {
    pub fn badge(&self) -> &'static str {
        match self {
            ChangeType::Create => "CREATED",
            ChangeType::Modify => "MODIFIED",
            ChangeType::Delete => "DELETED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineType {
    Header,
    Added,
    Removed,
    Context,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub id: String,
    pub tool: ToolType,
    pub session_id: String,
    pub session_title: String,
    pub timestamp: String,
    pub file_path: String,
    pub change_type: ChangeType,
    pub description: String,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub diff_lines: Vec<DiffLine>,
    pub target_tool_action: Option<String>,
}

impl CodeChange {
    pub fn matches_query(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        self.file_path.to_lowercase().contains(query_lower)
            || self.description.to_lowercase().contains(query_lower)
            || self.session_title.to_lowercase().contains(query_lower)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub tool: ToolType,
    pub title: String,
    pub timestamp: String,
    pub log_path: String,
    pub changes: Vec<CodeChange>,
    pub total_lines_added: usize,
    pub total_lines_deleted: usize,
}

impl ChatSession {
    pub fn matches_query(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }
        self.title.to_lowercase().contains(query_lower)
            || self.id.to_lowercase().contains(query_lower)
            || self.changes.iter().any(|c| c.matches_query(query_lower))
    }

    pub fn short_id(&self) -> &str {
        if self.id.len() > 8 {
            &self.id[..8]
        } else {
            &self.id
        }
    }

    pub fn short_timestamp(&self) -> &str {
        if self.timestamp.len() >= 19 {
            &self.timestamp[..19]
        } else {
            &self.timestamp
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_sessions: usize,
    pub total_changes: usize,
    pub total_lines_added: usize,
    pub total_lines_deleted: usize,
    pub unique_files: usize,
    pub count_antigravity: usize,
    pub count_claude: usize,
    pub count_codex: usize,
    pub count_gemini: usize,
}

impl ScanStats {
    pub fn net_loc_string(&self) -> String {
        format_net_loc(self.total_lines_added, self.total_lines_deleted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub file_path: String,
    pub filename: String,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub change_count: usize,
}

impl FileSummary {
    pub fn net_loc_string(&self) -> String {
        format_net_loc(self.lines_added, self.lines_deleted)
    }
}

pub fn format_net_loc(added: usize, deleted: usize) -> String {
    if added >= deleted {
        format!("+{}", added - deleted)
    } else {
        format!("-{}", deleted - added)
    }
}
