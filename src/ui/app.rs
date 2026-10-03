use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use egui::{Color32, Key, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::aggregator::compute_file_summaries;
use crate::models::{ChatSession, FileSummary, ScanStats, ToolType};
use crate::scanner::Scanner;
use crate::ui::agent_overview::AgentOverviewView;
use crate::ui::session_detail::{DetailAction, SessionDetailView};
use crate::ui::session_timeline::{SessionTimelineView, TimelineAction};
use crate::ui::theme::{Theme, ThemeMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiScene {
    /// Level 1 (Macro): Installed agents overview cards with aggregate changes
    AgentOverview,
    /// Level 2 (Middle): Chronological sessions timeline for selected agent
    AgentSessions { tool: ToolType },
    /// Level 3 (Micro): Selected session with file list and diff inspector
    SessionDetail { tool: ToolType, session_id: String },
}

#[derive(PartialEq, Eq)]
enum ActiveTab {
    ProgressiveFlow,
    AggregatedSummary,
}

type ScanResult = (Vec<ChatSession>, ScanStats);

pub struct DiffTrackApp {
    sessions: Vec<ChatSession>,
    stats: ScanStats,
    scanner: Scanner,
    is_scanning: bool,
    scan_rx: Option<Receiver<ScanResult>>,
    custom_scan_path: String,
    show_custom_scan: bool,

    // Theme Mode
    theme_mode: ThemeMode,

    // Progressive Zoom Scene State
    scene: GuiScene,
    session_search: String,
    selected_change_idx: usize,

    // Active View Tab
    active_tab: ActiveTab,

    // Window state
    is_fullscreen: bool,

    // Precomputed global summary data
    summary_files: Vec<FileSummary>,
    summary_search: String,
}

const JETBRAINS_MONO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Embed JetBrains Mono directly into binary (SIL Open Font License 1.1)
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        egui::FontData::from_static(JETBRAINS_MONO_REGULAR),
    );
    fonts.font_data.insert(
        "jetbrains_mono_bold".to_owned(),
        egui::FontData::from_static(JETBRAINS_MONO_BOLD),
    );

    // Prioritize JetBrains Mono for monospace (diffs, code, line numbers, file paths)
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());

    // Also prioritize JetBrains Mono for general UI text for clean developer workbench aesthetic
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());

    // 2. Cross-platform system fallback fonts (for CJK or system emoji)
    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_dir = PathBuf::from(windir).join("Fonts");
        for file in ["segoeui.ttf", "arial.ttf"] {
            if let Ok(data) = std::fs::read(font_dir.join(file)) {
                let name = file.to_string();
                fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data));
                fonts.families.entry(egui::FontFamily::Proportional).or_default().push(name.clone());
                fonts.families.entry(egui::FontFamily::Monospace).or_default().push(name);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_font_candidates = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];
        for path_str in linux_font_candidates {
            let path = PathBuf::from(path_str);
            if path.exists() {
                if let Ok(data) = std::fs::read(&path) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data));
                    fonts.families.entry(egui::FontFamily::Proportional).or_default().push(name);
                    break;
                }
            }
        }
    }

    ctx.set_fonts(fonts);
}

impl DiffTrackApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let scanner = Scanner::new();
        let (sessions, stats) = scanner.scan_all(None);
        let summary_files = compute_file_summaries(&sessions);

        Self {
            sessions,
            stats,
            scanner,
            is_scanning: false,
            scan_rx: None,
            custom_scan_path: String::new(),
            show_custom_scan: false,
            theme_mode: ThemeMode::Light,
            scene: GuiScene::AgentOverview,
            session_search: String::new(),
            selected_change_idx: 0,
            active_tab: ActiveTab::ProgressiveFlow,
            is_fullscreen: false,
            summary_files,
            summary_search: String::new(),
        }
    }

    fn trigger_scan(&mut self) {
        if self.is_scanning {
            return;
        }
        self.is_scanning = true;
        let (tx, rx): (Sender<ScanResult>, Receiver<ScanResult>) = channel();
        self.scan_rx = Some(rx);

        let custom_path = if !self.custom_scan_path.trim().is_empty() {
            Some(PathBuf::from(self.custom_scan_path.trim()))
        } else {
            None
        };

        thread::spawn(move || {
            let scanner = Scanner::new();
            let result = scanner.scan_all(custom_path.as_deref());
            let _ = tx.send(result);
        });
    }

    fn check_scan_results(&mut self) {
        if let Some(rx) = &self.scan_rx {
            if let Ok((sessions, stats)) = rx.try_recv() {
                self.summary_files = compute_file_summaries(&sessions);
                self.sessions = sessions;
                self.stats = stats;
                self.is_scanning = false;
                self.scan_rx = None;
                self.selected_change_idx = 0;
            }
        }
    }

    fn navigate_back(&mut self) {
        match self.scene {
            GuiScene::SessionDetail { tool, .. } => {
                self.scene = GuiScene::AgentSessions { tool };
                self.selected_change_idx = 0;
            }
            GuiScene::AgentSessions { .. } => {
                self.scene = GuiScene::AgentOverview;
                self.session_search.clear();
            }
            GuiScene::AgentOverview => {}
        }
    }
}

impl eframe::App for DiffTrackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_scan_results();

        let theme = Theme::from_mode(self.theme_mode);

        // Synchronize egui system visuals with the selected theme
        if self.theme_mode == ThemeMode::Light {
            ctx.set_visuals(egui::Visuals::light());
        } else {
            ctx.set_visuals(egui::Visuals::dark());
        }

        // Keyboard Shortcuts
        if ctx.input(|i| i.key_pressed(Key::F11)) {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }

        if ctx.input(|i| i.key_pressed(Key::F5)) {
            self.trigger_scan();
        }

        // Escape goes up one level in hierarchy
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.navigate_back();
        }

        // Top Panel: Unified Command Bar with Breadcrumbs & Theme Toggle
        egui::TopBottomPanel::top("top_header_panel")
            .frame(egui::Frame::none().fill(theme.bg_panel).inner_margin(egui::Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo + Brand Title
                    ui.label(RichText::new("⚡").size(16.0).color(theme.accent_blue));
                    ui.heading(RichText::new("DiffTrack").strong().size(15.0).color(theme.text_primary));
                    ui.label(RichText::new("WORKBENCH").size(10.0).strong().color(theme.text_muted));

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Hierarchical Breadcrumb Links in Header
                    if self.active_tab == ActiveTab::ProgressiveFlow {
                        let is_overview = self.scene == GuiScene::AgentOverview;

                        let overview_btn = egui::Button::new(
                            RichText::new("⌂ All Agents")
                                .size(11.5)
                                .strong()
                                .color(if is_overview { theme.text_primary } else { theme.accent_blue }),
                        )
                        .fill(Color32::TRANSPARENT);

                        let mut next_scene = None;
                        if ui.add(overview_btn).clicked() && !is_overview {
                            next_scene = Some(GuiScene::AgentOverview);
                        }

                        match &self.scene {
                            GuiScene::AgentOverview => {}
                            GuiScene::AgentSessions { tool } => {
                                ui.label(RichText::new("›").size(13.0).color(theme.text_gutter));
                                let rgb = tool.color_for_mode(self.theme_mode == ThemeMode::Light);
                                let tool_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                ui.colored_label(
                                    tool_color,
                                    RichText::new(format!("{} {}", tool.icon(), tool.display_name()))
                                        .strong()
                                        .size(11.5),
                                );
                            }
                            GuiScene::SessionDetail { tool, session_id } => {
                                ui.label(RichText::new("›").size(13.0).color(theme.text_gutter));
                                let rgb = tool.color_for_mode(self.theme_mode == ThemeMode::Light);
                                let tool_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);

                                let agent_btn = egui::Button::new(
                                    RichText::new(format!("{} {}", tool.icon(), tool.display_name()))
                                        .strong()
                                        .size(11.5)
                                        .color(tool_color),
                                )
                                .fill(Color32::TRANSPARENT);

                                let current_tool = *tool;
                                if ui.add(agent_btn).clicked() {
                                    next_scene = Some(GuiScene::AgentSessions { tool: current_tool });
                                }

                                ui.label(RichText::new("›").size(13.0).color(theme.text_gutter));
                                let short_id = if session_id.len() > 8 {
                                    &session_id[..8]
                                } else {
                                    session_id
                                };
                                ui.label(
                                    RichText::new(format!("Session #{}", short_id))
                                        .strong()
                                        .size(11.5)
                                        .color(theme.text_primary),
                                );
                            }
                        }

                        if let Some(s) = next_scene {
                            self.scene = s;
                        }
                    } else {
                        ui.label(
                            RichText::new("◫ Global File Summaries")
                                .strong()
                                .size(11.5)
                                .color(theme.text_primary),
                        );
                    }

                    // Right Actions Toolbar
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Theme Toggle Button
                        let mode_btn = egui::Button::new(
                            RichText::new(self.theme_mode.button_label())
                                .size(11.0)
                                .strong()
                                .color(theme.text_primary),
                        )
                        .fill(theme.bg_card)
                        .stroke(Stroke::new(1.0_f32, theme.border_card))
                        .rounding(4.0);

                        if ui.add(mode_btn).clicked() {
                            self.theme_mode = self.theme_mode.toggle();
                        }

                        // Fullscreen Toggle
                        let fs_label = if self.is_fullscreen { "🗗 Windowed" } else { "⤢ Fullscreen" };
                        if ui.button(RichText::new(fs_label).size(11.0)).clicked() {
                            self.is_fullscreen = !self.is_fullscreen;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                        }

                        // Rescan Button
                        let rescan_label = if self.is_scanning { "⟳ Scanning..." } else { "⟳ Rescan (F5)" };
                        if ui.add_enabled(!self.is_scanning, egui::Button::new(RichText::new(rescan_label).size(11.0))).clicked() {
                            self.trigger_scan();
                        }

                        // Custom Folder Toggle
                        let scan_toggle_text = if self.show_custom_scan { "◰ Close Folder" } else { "◰ Custom Scan..." };
                        if ui.button(RichText::new(scan_toggle_text).size(11.0)).clicked() {
                            self.show_custom_scan = !self.show_custom_scan;
                        }

                        ui.separator();

                        // View Tab Switcher
                        let summary_active = self.active_tab == ActiveTab::AggregatedSummary;
                        if ui.selectable_label(summary_active, RichText::new("◫ File Summary").size(11.5)).clicked() {
                            self.active_tab = ActiveTab::AggregatedSummary;
                        }

                        let flow_active = self.active_tab == ActiveTab::ProgressiveFlow;
                        if ui.selectable_label(flow_active, RichText::new("◈ Agent Flow").size(11.5)).clicked() {
                            self.active_tab = ActiveTab::ProgressiveFlow;
                        }
                    });
                });

                // Optional Collapsible Custom Scan Drawer
                if self.show_custom_scan {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Path:").color(theme.text_muted).size(11.5));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.custom_scan_path)
                                .hint_text("Enter repository or log directory path...")
                                .desired_width(420.0),
                        );
                        if ui.button("Scan Now").clicked() {
                            self.trigger_scan();
                        }

                        ui.separator();
                        ui.label(RichText::new("Discovered:").color(theme.text_muted).size(11.0));
                        let tool_pill = |ui: &mut Ui, tool: ToolType, count: usize| {
                            let rgb = tool.color_for_mode(self.theme_mode == ThemeMode::Light);
                            let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            ui.colored_label(color, RichText::new(format!("{}: {}", tool.badge_short(), count)).size(10.5).strong());
                        };
                        tool_pill(ui, ToolType::Antigravity, self.stats.count_antigravity);
                        tool_pill(ui, ToolType::ClaudeCode, self.stats.count_claude);
                        tool_pill(ui, ToolType::Codex, self.stats.count_codex);
                        tool_pill(ui, ToolType::GeminiCli, self.stats.count_gemini);
                    });
                }
            });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("bottom_status_panel")
            .frame(egui::Frame::none().fill(theme.bg_panel).inner_margin(egui::Margin::symmetric(14.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status_text = if self.is_scanning {
                        "Scanning AI agent logs across filesystem...".to_string()
                    } else {
                        format!(
                            "● Ready | {} sessions discovered across installed assistants | {} total code changes",
                            self.sessions.len(),
                            self.stats.total_changes
                        )
                    };
                    ui.label(RichText::new(status_text).color(theme.text_muted).size(11.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Esc Back · F11 Fullscreen · F5 Refresh").color(theme.text_muted).size(10.5));
                    });
                });
            });

        // Central Panel (Hierarchical Progressive Navigation)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme.bg_canvas).inner_margin(egui::Margin::symmetric(14.0, 10.0)))
            .show(ctx, |ui| {
                match self.active_tab {
                    ActiveTab::ProgressiveFlow => {
                        self.render_progressive_flow(ui, &theme);
                    }
                    ActiveTab::AggregatedSummary => {
                        self.render_summary_view(ui, &theme);
                    }
                }
            });
    }
}

impl DiffTrackApp {
    fn render_progressive_flow(&mut self, ui: &mut Ui, theme: &Theme) {
        match self.scene.clone() {
            GuiScene::AgentOverview => {
                if let Some(tool) = AgentOverviewView::show(ui, &self.sessions, &self.stats, &self.scanner, theme) {
                    self.scene = GuiScene::AgentSessions { tool };
                    self.session_search.clear();
                }
            }
            GuiScene::AgentSessions { tool } => {
                if let Some(action) = SessionTimelineView::show(ui, &self.sessions, tool, &mut self.session_search, theme) {
                    match action {
                        TimelineAction::BackToOverview => {
                            self.scene = GuiScene::AgentOverview;
                            self.session_search.clear();
                        }
                        TimelineAction::SelectSession(session_id) => {
                            self.scene = GuiScene::SessionDetail { tool, session_id };
                            self.selected_change_idx = 0;
                        }
                    }
                }
            }
            GuiScene::SessionDetail { tool, session_id } => {
                if let Some(session) = self.sessions.iter().find(|s| s.id == session_id) {
                    if let Some(action) = SessionDetailView::show(ui, session, &mut self.selected_change_idx, theme) {
                        match action {
                            DetailAction::BackToOverview => {
                                self.scene = GuiScene::AgentOverview;
                                self.session_search.clear();
                            }
                            DetailAction::BackToSessions => {
                                self.scene = GuiScene::AgentSessions { tool };
                            }
                        }
                    }
                } else {
                    // Session not found (perhaps deleted or after rescan)
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading(RichText::new("Session Not Found").color(theme.text_primary).size(16.0));
                            ui.add_space(8.0);
                            if ui.button("⮜ Return to Agents").clicked() {
                                self.scene = GuiScene::AgentOverview;
                            }
                        });
                    });
                }
            }
        }
    }

    fn render_summary_view(&mut self, ui: &mut Ui, theme: &Theme) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Header Bar for Summary Table
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Aggregated Code Changes").color(theme.text_primary).size(15.0));
                        ui.label(
                            RichText::new(format!("({} unique files touched)", self.summary_files.len()))
                                .color(theme.text_muted)
                                .size(11.5),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let search_frame = egui::Frame::none()
                                .fill(theme.bg_code)
                                .stroke(Stroke::new(1.0_f32, theme.border_subtle))
                                .rounding(4.0)
                                .inner_margin(Vec2::new(6.0, 3.0));

                            search_frame.show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.summary_search)
                                        .hint_text("Filter file paths...")
                                        .desired_width(240.0),
                                );
                            });
                            ui.label(RichText::new("⌕").size(13.0).strong().color(theme.text_muted));
                        });
                    });
                    ui.add_space(8.0);

                    let query = self.summary_search.to_lowercase();
                    let filtered: Vec<&FileSummary> = self.summary_files
                        .iter()
                        .filter(|f| query.is_empty() || f.file_path.to_lowercase().contains(&query))
                        .collect();

                    let table_frame = egui::Frame::none()
                        .fill(theme.bg_panel)
                        .stroke(Stroke::new(1.0_f32, theme.border_card))
                        .rounding(6.0)
                        .inner_margin(12.0);

                    table_frame.show(ui, |ui| {
                        egui::Grid::new("summary_table_grid")
                            .striped(true)
                            .num_columns(5)
                            .spacing([24.0, 9.0])
                            .min_col_width(70.0)
                            .show(ui, |ui| {
                                // Table Headers
                                ui.label(RichText::new("FILE").strong().size(11.0).color(theme.text_muted));
                                ui.label(RichText::new("FULL REPO PATH").strong().size(11.0).color(theme.text_muted));
                                ui.label(RichText::new("CHANGES").strong().size(11.0).color(theme.text_muted));
                                ui.label(RichText::new("LINES (+)").strong().size(11.0).color(theme.diff_add_text));
                                ui.label(RichText::new("LINES (-)").strong().size(11.0).color(theme.diff_del_text));
                                ui.end_row();

                                for item in filtered.into_iter().take(400) {
                                    ui.colored_label(
                                        theme.text_primary,
                                        RichText::new(&item.filename).monospace().strong().size(12.0),
                                    );

                                    let display_path = if item.file_path.len() > 65 {
                                        format!("...{}", &item.file_path[item.file_path.len() - 62..])
                                    } else {
                                        item.file_path.clone()
                                    };
                                    ui.label(RichText::new(display_path).monospace().color(theme.text_muted).size(11.0));

                                    ui.label(RichText::new(item.change_count.to_string()).size(12.0).color(theme.text_primary));

                                    ui.colored_label(
                                        theme.diff_add_text,
                                        RichText::new(format!("+{}", item.lines_added)).size(12.0).strong(),
                                    );

                                    ui.colored_label(
                                        theme.diff_del_text,
                                        RichText::new(format!("-{}", item.lines_deleted)).size(12.0).strong(),
                                    );

                                    ui.end_row();
                                }
                            });
                    });
                });
            });
    }
}
