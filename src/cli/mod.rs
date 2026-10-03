use std::path::PathBuf;

use crate::aggregator::{
    compute_file_summaries, filter_sessions, find_session, group_sessions_by_agent,
};
use crate::models::{ChatSession, DiffLineType, ScanStats, ToolType};
use crate::scanner::Scanner;

// ANSI escape codes for terminal coloring
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";

fn tool_ansi_info(tool: ToolType) -> (&'static str, &'static str, &'static str) {
    match tool {
        ToolType::Antigravity => ("[ANTIGRAVITY]", "ANTIGRAVITY", MAGENTA),
        ToolType::ClaudeCode => ("[CLAUDE]", "CLAUDE", BLUE),
        ToolType::Codex => ("[CODEX]", "CODEX", YELLOW),
        ToolType::GeminiCli => ("[GEMINI]", "GEMINI", CYAN),
        ToolType::Generic => ("[GENERIC]", "GENERIC", DIM),
    }
}

pub fn run_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut custom_scan_path: Option<PathBuf> = None;
    let mut tool_filter: Option<ToolType> = None;
    let mut search_query = String::new();
    let mut limit: usize = 30;
    let mut as_json = false;
    let mut is_simple = false;
    let mut by_agent = false;

    // Detect command or flags
    let mut command = "summary".to_string(); // default CLI command
    let mut diff_target_id: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" | "help" => {
                print_help();
                return Ok(());
            }
            "-s" | "--simple" => {
                is_simple = true;
            }
            "--stats" | "stats" => {
                command = "stats".to_string();
            }
            "--summary" | "summary" => {
                command = "summary".to_string();
            }
            "--sessions" | "sessions" | "list" | "--list" => {
                command = "sessions".to_string();
            }
            "--agents" | "agents" | "--by-agent" | "by-agent" | "--by-tool" | "by-tool" => {
                command = "agents".to_string();
            }
            "-b" => {
                by_agent = true;
            }
            "--diff" | "diff" => {
                command = "diff".to_string();
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    diff_target_id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--scan" | "scan" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    custom_scan_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--json" | "-j" => {
                as_json = true;
            }
            "--tool" | "-t" => {
                if i + 1 < args.len() {
                    let t = args[i + 1].to_lowercase();
                    tool_filter = match t.as_str() {
                        "antigravity" | "agy" => Some(ToolType::Antigravity),
                        "claude" | "claude-code" => Some(ToolType::ClaudeCode),
                        "codex" | "copilot" => Some(ToolType::Codex),
                        "gemini" | "gemini-cli" => Some(ToolType::GeminiCli),
                        "generic" | "diff" | "other" => Some(ToolType::Generic),
                        _ => None,
                    };
                    i += 1;
                }
            }
            "--limit" | "-n" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<usize>() {
                        limit = n;
                    }
                    i += 1;
                }
            }
            "--query" | "-q" => {
                if i + 1 < args.len() {
                    search_query = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {
                // If positional argument doesn't start with -, treat as query or target
                if !arg.starts_with('-') {
                    if diff_target_id.is_none() && command == "diff" {
                        diff_target_id = Some(arg.to_string());
                    } else if search_query.is_empty() {
                        search_query = arg.to_string();
                    }
                }
            }
        }
        i += 1;
    }

    if by_agent {
        command = "agents".to_string();
    }

    // Run scanner (suppress banner when outputting pure json or simple mode)
    if !as_json && !is_simple {
        eprintln!(
            "{}{}Agent Diff Tracker CLI{} - Scanning AI sessions (Antigravity, Claude, Codex, Gemini)...{}",
            BOLD, CYAN, DIM, RESET
        );
    }

    let scanner = Scanner::new();
    let (sessions, stats) = scanner.scan_all(custom_scan_path.as_deref());

    if as_json {
        if command == "stats" {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        } else if command == "summary" {
            let files = compute_file_summaries(&sessions);
            println!("{}", serde_json::to_string_pretty(&files)?);
        } else if command == "agents" {
            let groups = group_sessions_by_agent(&sessions, &search_query);
            println!("{}", serde_json::to_string_pretty(&groups)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&sessions)?);
        }
        return Ok(());
    }

    match command.as_str() {
        "stats" => print_stats(&stats, is_simple),
        "sessions" => print_sessions(&sessions, tool_filter, &search_query, limit, is_simple),
        "agents" => print_agents(&sessions, &search_query, limit, is_simple),
        "diff" => {
            if let Some(target_id) = diff_target_id {
                print_diff(&sessions, &target_id, is_simple);
            } else {
                eprintln!(
                    "{}Error: Please provide a session ID. Example: diff-track diff <session-id>{}{}",
                    RED,
                    if is_simple { " --simple" } else { "" },
                    RESET
                );
            }
        }
        _ => {
            if !is_simple {
                print_stats(&stats, false);
                println!();
            }
            print_file_summary(&sessions, &search_query, limit, is_simple, Some(&stats));
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"{bold}Agent Diff Tracker CLI - AI Code Change Aggregator{reset}
Summarize and inspect lines of code (LOC) changed across Antigravity, Claude Code, Codex, and Gemini CLI.

{cyan}{bold}USAGE:{reset}
    diff-track [OPTIONS] [COMMAND]

{cyan}{bold}COMMANDS:{reset}
    {green}summary{reset}               Show aggregated file change summary table (default)
    {green}stats{reset}                 Show overall metrics & tool breakdown
    {green}sessions, list{reset}        List chat sessions with prompt titles and LOC changes
    {green}agents, by-agent{reset}      List all sessions grouped by AI Agent / Tool
    {green}diff <SESSION_ID>{reset}     Display syntax-highlighted unified diff for a session
    {green}scan <PATH>{reset}           Scan a custom folder for AI logs and report changes

{cyan}{bold}OPTIONS:{reset}
    {yellow}--gui, -g{reset}             Launch the Fullscreen GUI application
    {yellow}--simple, -s{reset}          Condensed, structured output without decorative borders
    {yellow}--by-agent, -b{reset}        Group sessions by AI Agent (or use 'agents' command)
    {yellow}--tool, -t <NAME>{reset}     Filter by tool (antigravity, claude, codex, gemini, generic)
    {yellow}--query, -q <TEXT>{reset}    Search sessions, prompts, or file paths
    {yellow}--limit, -n <NUM>{reset}     Limit output rows per group/table (default: 30)
    {yellow}--json, -j{reset}            Output results as formatted JSON
    {yellow}--help, -h{reset}            Show this help reference

{cyan}{bold}EXAMPLES:{reset}
    diff-track summary
    diff-track summary --simple
    diff-track sessions --simple -n 10
    diff-track agents
    diff-track agents --simple
    diff-track sessions --by-agent
    diff-track stats --simple
    diff-track diff 858a00b3 --simple
    diff-track scan C:\Projects\MyRepo
    diff-track --json agents
"#,
        bold = BOLD,
        reset = RESET,
        cyan = CYAN,
        green = GREEN,
        yellow = YELLOW,
    );
}

fn print_stats(stats: &ScanStats, is_simple: bool) {
    let net = stats.net_loc_string();

    if is_simple {
        println!("sessions: {}", stats.total_sessions);
        println!("changes: {}", stats.total_changes);
        println!("files: {}", stats.unique_files);
        println!("added: +{}", stats.total_lines_added);
        println!("deleted: -{}", stats.total_lines_deleted);
        println!("net: {}", net);
        println!(
            "tools: antigravity={} claude={} codex={} gemini={}",
            stats.count_antigravity, stats.count_claude, stats.count_codex, stats.count_gemini
        );
        return;
    }

    println!("{bold}================================================================================{reset}", bold = BOLD, reset = RESET);
    println!("  {bold}{cyan}Agent Diff Tracker AI LOC Dashboard{reset}", bold = BOLD, cyan = CYAN, reset = RESET);
    println!("{bold}================================================================================{reset}", bold = BOLD, reset = RESET);
    println!(
        "  Total Sessions:   {bold}{:<8}{reset} | Total Code Changes: {bold}{:<8}{reset}",
        stats.total_sessions, stats.total_changes, bold = BOLD, reset = RESET
    );
    println!(
        "  Lines Added (+):  {bold}{green}+{:<7}{reset} | Lines Deleted (-):  {bold}{red}-{:<7}{reset} | Net LOC: {bold}{cyan}{}{reset}",
        stats.total_lines_added, stats.total_lines_deleted, net,
        bold = BOLD, green = GREEN, red = RED, cyan = CYAN, reset = RESET
    );
    println!("  Unique Files:     {bold}{:<8}{reset}", stats.unique_files, bold = BOLD, reset = RESET);
    println!("{dim}--------------------------------------------------------------------------------{reset}", dim = DIM, reset = RESET);
    println!(
        "  Tool Breakdown:   {magenta}Antigravity:{reset} {:<4} | {blue}Claude Code:{reset} {:<4} | {yellow}Codex:{reset} {:<4} | {cyan}Gemini CLI:{reset} {:<4}",
        stats.count_antigravity, stats.count_claude, stats.count_codex, stats.count_gemini,
        magenta = MAGENTA, blue = BLUE, yellow = YELLOW, cyan = CYAN, reset = RESET
    );
    println!("{bold}================================================================================{reset}", bold = BOLD, reset = RESET);
}

fn print_file_summary(
    sessions: &[ChatSession],
    query: &str,
    limit: usize,
    is_simple: bool,
    stats: Option<&ScanStats>,
) {
    let mut files = compute_file_summaries(sessions);

    if !query.trim().is_empty() {
        let q = query.to_lowercase();
        files.retain(|f| f.filename.to_lowercase().contains(&q) || f.file_path.to_lowercase().contains(&q));
    }

    if is_simple {
        if let Some(s) = stats {
            println!(
                "# TOTAL: sessions={} changes={} files={} lines=+{}/-{} net={}",
                s.total_sessions, s.total_changes, s.unique_files, s.total_lines_added, s.total_lines_deleted, s.net_loc_string()
            );
        }
        for f in files.iter().take(limit) {
            println!(
                "+{:<5} -{:<5} ({:>5}) {:>3} chg  {:<28}  {}",
                f.lines_added, f.lines_deleted, f.net_loc_string(), f.change_count, f.filename, f.file_path
            );
        }
        return;
    }

    println!(
        "{bold}{cyan}Top Modified Files ({}/{}):{reset}",
        files.len().min(limit), files.len(),
        bold = BOLD, cyan = CYAN, reset = RESET
    );
    println!(
        "{dim}{:<4} {:<28} {:>7} {:>9} {:>9} {:>9}  {:<30}{reset}",
        "#", "FILENAME", "CHANGES", "+ADDED", "-DELETED", "NET LOC", "FULL PATH",
        dim = DIM, reset = RESET
    );
    println!("{dim}----------------------------------------------------------------------------------------------------{reset}", dim = DIM, reset = RESET);

    for (idx, f) in files.iter().take(limit).enumerate() {
        let net = if f.lines_added >= f.lines_deleted {
            format!("{green}{:<8}{reset}", f.net_loc_string(), green = GREEN, reset = RESET)
        } else {
            format!("{red}{:<8}{reset}", f.net_loc_string(), red = RED, reset = RESET)
        };

        let display_name = if f.filename.len() > 28 {
            format!("{}...", &f.filename[..25])
        } else {
            f.filename.clone()
        };

        let display_path = if f.file_path.len() > 45 {
            format!("...{}", &f.file_path[f.file_path.len() - 42..])
        } else {
            f.file_path.clone()
        };

        println!(
            "{:<4} {bold}{:<28}{reset} {:>7} {green}{:>9}{reset} {red}{:>9}{reset} {:>9}  {dim}{:<45}{reset}",
            idx + 1, display_name, f.change_count,
            format!("+{}", f.lines_added), format!("-{}", f.lines_deleted),
            net, display_path,
            bold = BOLD, reset = RESET, green = GREEN, red = RED, dim = DIM
        );
    }
}

fn print_sessions(
    sessions: &[ChatSession],
    tool_filter: Option<ToolType>,
    search_query: &str,
    limit: usize,
    is_simple: bool,
) {
    let filtered = filter_sessions(sessions, tool_filter, search_query);

    if is_simple {
        for s in filtered.iter().take(limit) {
            let (_, simple_badge, _) = tool_ansi_info(s.tool);
            println!(
                "{:<8}  {:<19}  {:<11}  +{} -{}  {}",
                s.short_id(), s.short_timestamp(), simple_badge,
                s.total_lines_added, s.total_lines_deleted, s.title
            );
        }
        return;
    }

    println!(
        "{bold}{cyan}AI Chat Sessions ({}/{}):{reset}",
        filtered.len().min(limit), filtered.len(),
        bold = BOLD, cyan = CYAN, reset = RESET
    );
    println!(
        "{dim}{:<14} {:<19} {:<10} {:>8} {:>8}  {:<40}{reset}",
        "TOOL", "TIMESTAMP", "SESSION ID", "+ADDED", "-DELETED", "PROMPT / TITLE",
        dim = DIM, reset = RESET
    );
    println!("{dim}----------------------------------------------------------------------------------------------------{reset}", dim = DIM, reset = RESET);

    for s in filtered.iter().take(limit) {
        let (badge, _, color) = tool_ansi_info(s.tool);
        let title_clean: String = s.title.chars().take(45).collect();

        println!(
            "{color}{bold}{:<14}{reset} {:<19} {dim}{:<10}{reset} {green}{:>8}{reset} {red}{:>8}{reset}  {bold}{:<45}{reset}",
            badge, s.short_timestamp(), s.short_id(),
            format!("+{}", s.total_lines_added), format!("-{}", s.total_lines_deleted),
            title_clean,
            color = color, bold = BOLD, reset = RESET, dim = DIM, green = GREEN, red = RED
        );
    }
}

fn print_agents(sessions: &[ChatSession], search_query: &str, limit_per_agent: usize, is_simple: bool) {
    let groups = group_sessions_by_agent(sessions, search_query);

    if groups.is_empty() {
        if !is_simple {
            println!("{}No sessions found matching query '{}'{}", YELLOW, search_query, RESET);
        }
        return;
    }

    if is_simple {
        for group in &groups {
            let (_, simple_badge, _) = tool_ansi_info(group.tool);
            println!(
                "# AGENT: {} ({}) - sessions={} added=+{} deleted=-{}",
                simple_badge, group.tool_name, group.total_sessions, group.total_lines_added, group.total_lines_deleted
            );
            for s in group.sessions.iter().take(limit_per_agent) {
                println!(
                    "{:<8}  {:<19}  +{:<5} -{:<5}  {}",
                    s.short_id(), s.short_timestamp(), s.total_lines_added, s.total_lines_deleted, s.title
                );
            }
        }
        return;
    }

    println!(
        "{bold}{cyan}AI Chat Sessions Grouped by Agent ({search_info}):{reset}",
        search_info = if search_query.is_empty() {
            format!("{} agents active", groups.len())
        } else {
            format!("filter: '{}', {} agents matched", search_query, groups.len())
        },
        bold = BOLD, cyan = CYAN, reset = RESET
    );

    for group in &groups {
        let (badge, _, color) = tool_ansi_info(group.tool);

        println!();
        println!(
            "{color}{bold}{badge} {name}{reset} {dim}—{reset} {bold}{count}{reset} session(s) | {green}+{added}{reset} / {red}-{deleted}{reset} lines",
            badge = badge, name = group.tool_name, count = group.total_sessions,
            added = group.total_lines_added, deleted = group.total_lines_deleted,
            color = color, bold = BOLD, dim = DIM, green = GREEN, red = RED, reset = RESET
        );
        println!(
            "{dim}{:<19} {:<10} {:>8} {:>8}  {:<45}{reset}",
            "TIMESTAMP", "SESSION ID", "+ADDED", "-DELETED", "PROMPT / TITLE",
            dim = DIM, reset = RESET
        );
        println!("{dim}----------------------------------------------------------------------------------------------------{reset}", dim = DIM, reset = RESET);

        for s in group.sessions.iter().take(limit_per_agent) {
            let title_clean: String = s.title.chars().take(45).collect();
            println!(
                "{:<19} {dim}{:<10}{reset} {green}{:>8}{reset} {red}{:>8}{reset}  {bold}{:<45}{reset}",
                s.short_timestamp(), s.short_id(),
                format!("+{}", s.total_lines_added), format!("-{}", s.total_lines_deleted),
                title_clean,
                bold = BOLD, reset = RESET, dim = DIM, green = GREEN, red = RED
            );
        }

        if group.sessions.len() > limit_per_agent {
            println!(
                "  {dim}... and {} more session(s) (use -n {} to show all){reset}",
                group.sessions.len() - limit_per_agent, group.sessions.len(),
                dim = DIM, reset = RESET
            );
        }
    }
}

fn print_diff(sessions: &[ChatSession], id_query: &str, is_simple: bool) {
    let session = match find_session(sessions, id_query) {
        Some(s) => s,
        None => {
            eprintln!("{}Error: Session '{}' not found{}", if is_simple { "" } else { RED }, id_query, if is_simple { "" } else { RESET });
            return;
        }
    };

    if is_simple {
        for change in &session.changes {
            println!("--- a/{}", change.file_path);
            println!("+++ b/{}", change.file_path);
            for line in &change.diff_lines {
                match line.line_type {
                    DiffLineType::Added => println!("+{}", line.text),
                    DiffLineType::Removed => println!("-{}", line.text),
                    DiffLineType::Header => println!("{}", line.text),
                    DiffLineType::Context => println!(" {}", line.text),
                }
            }
        }
        return;
    }

    println!(
        "{bold}{cyan}Session:{reset} {} ({}) | {bold}Date:{reset} {} | {green}+{} lines{reset} / {red}-{} lines{reset}",
        session.title, session.id, session.timestamp,
        session.total_lines_added, session.total_lines_deleted,
        bold = BOLD, cyan = CYAN, reset = RESET, green = GREEN, red = RED
    );
    println!("{dim}Log Path: {}{reset}", session.log_path, dim = DIM, reset = RESET);
    println!("{bold}================================================================================{reset}", bold = BOLD, reset = RESET);

    for change in &session.changes {
        println!(
            "\n{bold}File: {cyan}{}{reset} [{yellow}{}{reset}] (+{} / -{})",
            change.file_path, change.change_type.badge(),
            change.lines_added, change.lines_deleted,
            bold = BOLD, cyan = CYAN, reset = RESET, yellow = YELLOW
        );

        for line in &change.diff_lines {
            match line.line_type {
                DiffLineType::Added => println!("{green}+ {}{reset}", line.text, green = GREEN, reset = RESET),
                DiffLineType::Removed => println!("{red}- {}{reset}", line.text, red = RED, reset = RESET),
                DiffLineType::Header => println!("{cyan}{}{reset}", line.text, cyan = CYAN, reset = RESET),
                DiffLineType::Context => println!("  {}", line.text),
            }
        }
    }
}
