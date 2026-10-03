pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod generic_diff;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::{ChatSession, ScanStats};
use self::antigravity::parse_antigravity_transcript;
use self::claude::parse_claude_log;
use self::codex::parse_codex_log;
use self::gemini::parse_gemini_log;

pub struct Scanner {
    user_home: PathBuf,
}

impl Scanner {
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        Self { user_home: home }
    }

    /// Check if an AI agent is installed or configured on this machine
    pub fn is_agent_installed(&self, tool: crate::models::ToolType) -> bool {
        match tool {
            crate::models::ToolType::Antigravity => {
                self.user_home.join(".gemini").join("antigravity-cli").exists()
                    || self.user_home.join(".gemini").join("antigravity").exists()
            }
            crate::models::ToolType::ClaudeCode => {
                self.user_home.join(".claude").exists()
                    || self.user_home.join(".claude-code").exists()
                    || self.user_home.join("AppData").join("Roaming").join("Claude").exists()
                    || self.user_home.join("AppData").join("Local").join("claude-code").exists()
            }
            crate::models::ToolType::Codex => {
                self.user_home.join(".codex").exists()
                    || self.user_home.join(".copilot").exists()
                    || self.user_home.join("AppData").join("Local").join("github-copilot").exists()
            }
            crate::models::ToolType::GeminiCli => {
                self.user_home.join(".gemini").join("gemini-cli").exists()
                    || self.user_home.join(".gemini").join("history").exists()
                    || self.user_home.join("AppData").join("Roaming").join("Gemini").exists()
            }
            crate::models::ToolType::Generic => false,
        }
    }

    /// Full scan of all default AI tool locations + optional custom path
    pub fn scan_all(&self, custom_path: Option<&Path>) -> (Vec<ChatSession>, ScanStats) {
        let mut all_sessions = Vec::new();
        let mut seen_session_ids = HashSet::new();

        let mut add_sessions = |sessions: Vec<ChatSession>| {
            for s in sessions {
                if seen_session_ids.insert(format!("{}_{}", s.tool.display_name(), s.id)) {
                    all_sessions.push(s);
                }
            }
        };

        add_sessions(self.scan_antigravity());
        add_sessions(self.scan_claude());
        add_sessions(self.scan_codex());
        add_sessions(self.scan_gemini());

        if let Some(cpath) = custom_path {
            if cpath.exists() {
                add_sessions(self.scan_custom_directory(cpath));
            }
        }

        // Sort sessions by timestamp descending (newest first)
        all_sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Calculate statistics
        let stats = compute_stats(&all_sessions);

        (all_sessions, stats)
    }

    fn scan_antigravity(&self) -> Vec<ChatSession> {
        let mut sessions = Vec::new();
        let brain_dir = self.user_home.join(".gemini").join("antigravity-cli").join("brain");
        if brain_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&brain_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let session_id = entry.file_name().to_string_lossy().to_string();
                        // Prefer transcript.jsonl or transcript_full.jsonl
                        let candidates = [
                            path.join(".system_generated").join("logs").join("transcript.jsonl"),
                            path.join(".system_generated").join("logs").join("transcript_full.jsonl"),
                            path.join("transcript.jsonl"),
                        ];

                        for c in candidates {
                            if c.exists() {
                                if let Some(session) = parse_antigravity_transcript(&c, &session_id) {
                                    sessions.push(session);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Also check .gemini/antigravity/conversations
        let conv_dir = self.user_home.join(".gemini").join("antigravity").join("conversations");
        if conv_dir.exists() {
            self.scan_directory_with_parser(&conv_dir, &mut sessions, parse_antigravity_transcript);
        }

        sessions
    }

    fn scan_claude(&self) -> Vec<ChatSession> {
        let mut sessions = Vec::new();
        let mut search_dirs = vec![
            self.user_home.join(".claude").join("projects"),
            self.user_home.join(".claude").join("sessions"),
            self.user_home.join(".claude").join("history"),
            self.user_home.join(".claude"),
            self.user_home.join(".claude-code"),
            self.user_home.join(".teach.claude"),
            self.user_home.join("AppData").join("Roaming").join("Claude"),
            self.user_home.join("AppData").join("Roaming").join("claude-code"),
            self.user_home.join("AppData").join("Local").join("claude-code"),
            self.user_home.join("AppData").join("Local").join("Claude"),
        ];

        if let Ok(val) = std::env::var("CLAUDE_CONFIG_DIR") {
            if !val.is_empty() {
                search_dirs.push(PathBuf::from(val));
            }
        }

        for dir in search_dirs {
            if dir.exists() {
                self.scan_directory_with_parser(&dir, &mut sessions, parse_claude_log);
            }
        }

        sessions
    }

    fn scan_codex(&self) -> Vec<ChatSession> {
        let mut sessions = Vec::new();
        let codex_home = std::env::var("CODEX_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.user_home.join(".codex"));

        let search_dirs = [
            codex_home.join("sessions"),
            codex_home.join("archived_sessions"),
            codex_home,
            self.user_home.join(".copilot"),
            self.user_home.join(".copilot").join("logs"),
            self.user_home.join(".config").join("github-copilot"),
            self.user_home.join("AppData").join("Local").join("github-copilot"),
            self.user_home.join("AppData").join("Roaming").join("GitHub Copilot"),
        ];

        for dir in search_dirs {
            if dir.exists() {
                self.scan_directory_with_parser(&dir, &mut sessions, parse_codex_log);
            }
        }

        sessions
    }

    fn scan_gemini(&self) -> Vec<ChatSession> {
        let mut sessions = Vec::new();
        let search_dirs = [
            self.user_home.join(".gemini").join("tmp"),
            self.user_home.join(".gemini").join("history"),
            self.user_home.join(".gemini").join("gemini-cli"),
            self.user_home.join("AppData").join("Roaming").join("Gemini"),
            self.user_home.join("AppData").join("Local").join("gemini"),
        ];

        for dir in search_dirs {
            if dir.exists() {
                self.scan_directory_with_parser(&dir, &mut sessions, parse_gemini_log);
            }
        }

        sessions
    }

    pub fn scan_custom_directory(&self, path: &Path) -> Vec<ChatSession> {
        let mut sessions = Vec::new();
        for entry in WalkDir::new(path).max_depth(6).into_iter().flatten() {
            let p = entry.path();
            if p.is_file() {
                let file_name = p.file_name().unwrap_or_default().to_string_lossy();
                let ext = p.extension().unwrap_or_default().to_string_lossy().to_lowercase();

                if ext == "jsonl" || ext == "json" || ext == "log" || ext == "diff" || ext == "patch" {
                    let session_id = p.file_stem().unwrap_or_default().to_string_lossy().to_string();

                    // Try in sequence: Antigravity -> Claude -> Codex -> Gemini
                    if file_name.contains("transcript") {
                        if let Some(s) = parse_antigravity_transcript(p, &session_id) {
                            sessions.push(s);
                            continue;
                        }
                    }
                    if let Some(s) = parse_claude_log(p, &session_id) {
                        sessions.push(s);
                        continue;
                    }
                    if let Some(s) = parse_codex_log(p, &session_id) {
                        sessions.push(s);
                        continue;
                    }
                    if let Some(s) = parse_gemini_log(p, &session_id) {
                        sessions.push(s);
                        continue;
                    }
                }
            }
        }
        sessions
    }

    fn scan_directory_with_parser<F>(&self, root: &Path, sessions: &mut Vec<ChatSession>, parser: F)
    where
        F: Fn(&Path, &str) -> Option<ChatSession>,
    {
        for entry in WalkDir::new(root).max_depth(6).into_iter().flatten() {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                if ext == "jsonl" || ext == "json" || ext == "log" || ext == "diff" || ext == "patch" || ext == "txt" {
                    let id = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    if let Some(s) = parser(p, &id) {
                        sessions.push(s);
                    }
                }
            }
        }
    }
}

pub use crate::aggregator::compute_stats;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_runs_without_panicking() {
        let scanner = Scanner::new();
        let (_sessions, stats) = scanner.scan_all(None);
        println!("=== AGENT DIFF TRACKER SCANNER TEST RESULTS ===");
        println!("Total Sessions Discovered: {}", stats.total_sessions);
        println!("Total Code Changes:        {}", stats.total_changes);
        println!("Total Lines Added (+):     {}", stats.total_lines_added);
        println!("Total Lines Deleted (-):   {}", stats.total_lines_deleted);
        println!("Total Unique Files:        {}", stats.unique_files);
        println!("=======================================");
        // Ensure stats calculation finishes cleanly
        assert_eq!(stats.total_lines_added + stats.total_lines_deleted, stats.total_lines_added + stats.total_lines_deleted);
    }

    #[test]
    fn test_scanner_custom_mock_directory() {
        let temp_dir = std::env::temp_dir().join(format!("difftrack_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mock_log = temp_dir.join("mock_transcript.jsonl");
        let sample_entry = r#"{"type":"USER_INPUT","created_at":"2026-10-01T12:00:00Z","content":"<USER_REQUEST>Add new feature</USER_REQUEST>"}
{"type":"PLANNER_RESPONSE","created_at":"2026-10-01T12:00:05Z","tool_calls":[{"name":"write_to_file","args":{"TargetFile":"src/test.rs","CodeContent":"fn test() {}\nfn hello() {}\n","Description":"Create test file","toolAction":"write_to_file"}}]}
"#;
        std::fs::write(&mock_log, sample_entry).unwrap();

        let scanner = Scanner::new();
        let sessions = scanner.scan_custom_directory(&temp_dir);
        let stats = compute_stats(&sessions);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(sessions.len(), 1, "Mock transcript should be discovered and parsed as 1 session");
        assert_eq!(stats.total_changes, 1, "Mock session should have 1 code change");
        assert_eq!(stats.total_lines_added, 2, "Mock session should have 2 added lines");
        assert_eq!(sessions[0].title, "Add new feature");
    }
}

