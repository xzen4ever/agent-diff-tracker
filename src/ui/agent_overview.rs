use egui::{Color32, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::models::{format_net_loc, ChatSession, ScanStats, ToolType};
use crate::scanner::Scanner;
use crate::ui::theme::{Theme, ThemeMode};

pub struct AgentOverviewView;

impl AgentOverviewView {
    pub fn show(
        ui: &mut Ui,
        sessions: &[ChatSession],
        stats: &ScanStats,
        scanner: &Scanner,
        theme: &Theme,
    ) -> Option<ToolType> {
        let mut selected_agent = None;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // Section Title & Hero Banner
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("◈").size(20.0).color(theme.accent_blue));
                            ui.heading(
                                RichText::new("Discovered AI Coding Agents")
                                    .strong()
                                    .size(20.0)
                                    .color(theme.text_primary),
                            );
                        });
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(
                                "Macro overview of installed AI coding assistants and cumulative filesystem code modifications.",
                            )
                            .color(theme.text_muted)
                            .size(12.5),
                        );
                    });
                });

                ui.add_space(16.0);

                // Global Aggregate Metrics Dashboard Banner
                let banner_frame = egui::Frame::none()
                    .fill(theme.bg_panel)
                    .stroke(Stroke::new(1.0_f32, theme.border_card))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0));

                banner_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        metric_stat(ui, "TOTAL SESSIONS", &stats.total_sessions.to_string(), theme.text_primary, theme.text_muted);
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(24.0);

                        metric_stat(ui, "CODE CHANGES", &stats.total_changes.to_string(), theme.text_primary, theme.text_muted);
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(24.0);

                        metric_stat(ui, "LINES ADDED", &format!("+{}", stats.total_lines_added), theme.diff_add_text, theme.text_muted);
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(24.0);

                        metric_stat(ui, "LINES DELETED", &format!("-{}", stats.total_lines_deleted), theme.diff_del_text, theme.text_muted);
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(24.0);

                        let net_loc = format_net_loc(stats.total_lines_added, stats.total_lines_deleted);
                        let net_color = if stats.total_lines_added >= stats.total_lines_deleted {
                            theme.diff_add_text
                        } else {
                            theme.diff_del_text
                        };
                        metric_stat(ui, "NET LOC DELTA", &net_loc, net_color, theme.text_muted);
                        ui.add_space(24.0);
                        ui.separator();
                        ui.add_space(24.0);

                        metric_stat(ui, "UNIQUE FILES TOUCHED", &stats.unique_files.to_string(), theme.accent_blue, theme.text_muted);
                    });
                });

                ui.add_space(20.0);

                // Section Subtitle
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("INSTALLED AGENT PLATFORMS")
                            .strong()
                            .size(11.5)
                            .color(theme.text_muted),
                    );
                    ui.label(RichText::new("— Select an agent below to inspect session timeline and diffs").size(11.0).color(theme.text_gutter));
                });

                ui.add_space(8.0);

                // Agent Cards Grid
                let candidate_tools = [
                    ToolType::Antigravity,
                    ToolType::ClaudeCode,
                    ToolType::Codex,
                    ToolType::GeminiCli,
                    ToolType::Generic,
                ];

                for tool in candidate_tools {
                    let tool_sessions: Vec<&ChatSession> = sessions.iter().filter(|s| s.tool == tool).collect();
                    let session_count = tool_sessions.len();
                    let is_installed = scanner.is_agent_installed(tool) || session_count > 0;

                    // If Generic has 0 sessions and is not installed, hide it
                    if tool == ToolType::Generic && session_count == 0 {
                        continue;
                    }

                    let lines_added: usize = tool_sessions.iter().map(|s| s.total_lines_added).sum();
                    let lines_deleted: usize = tool_sessions.iter().map(|s| s.total_lines_deleted).sum();
                    let total_changes: usize = tool_sessions.iter().map(|s| s.changes.len()).sum();
                    let latest_session = tool_sessions.first();

                    let tool_rgb = tool.color_for_mode(theme.mode == ThemeMode::Light);
                    let tool_color = Color32::from_rgb(tool_rgb[0], tool_rgb[1], tool_rgb[2]);

                    let card_frame = egui::Frame::none()
                        .fill(theme.bg_card)
                        .stroke(Stroke::new(1.0_f32, theme.border_card))
                        .rounding(6.0)
                        .inner_margin(egui::Margin::symmetric(14.0, 12.0));

                    let card_resp = card_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Tool Badge
                            let badge_bg = Color32::from_rgba_premultiplied(tool_rgb[0], tool_rgb[1], tool_rgb[2], 25);
                            egui::Frame::none()
                                .fill(badge_bg)
                                .rounding(4.0)
                                .inner_margin(Vec2::new(8.0, 3.0))
                                .show(ui, |ui| {
                                    ui.colored_label(
                                        tool_color,
                                        RichText::new(format!("{} {}", tool.icon(), tool.badge_short()))
                                            .strong()
                                            .size(11.5),
                                    );
                                });

                            ui.add_space(4.0);

                            // Tool Name
                            ui.heading(
                                RichText::new(tool.display_name())
                                    .strong()
                                    .size(15.0)
                                    .color(theme.text_primary),
                            );

                            // Status Pill
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if session_count > 0 {
                                    let pill_bg = Color32::from_rgba_premultiplied(46, 160, 67, 25);
                                    egui::Frame::none()
                                        .fill(pill_bg)
                                        .rounding(4.0)
                                        .inner_margin(Vec2::new(8.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.colored_label(theme.accent_green, RichText::new("●").size(9.0));
                                                ui.colored_label(
                                                    theme.diff_add_text,
                                                    RichText::new(format!("ACTIVE ({} SESSIONS)", session_count))
                                                        .strong()
                                                        .size(10.5),
                                                );
                                            });
                                        });
                                } else if is_installed {
                                    let pill_bg = Color32::from_rgba_premultiplied(56, 139, 253, 25);
                                    egui::Frame::none()
                                        .fill(pill_bg)
                                        .rounding(4.0)
                                        .inner_margin(Vec2::new(8.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.colored_label(theme.accent_blue, RichText::new("●").size(9.0));
                                                ui.colored_label(
                                                    theme.accent_blue,
                                                    RichText::new("INSTALLED (0 LOGS)")
                                                        .strong()
                                                        .size(10.5),
                                                );
                                            });
                                        });
                                } else {
                                    egui::Frame::none()
                                        .fill(Color32::from_rgba_premultiplied(100, 110, 125, 20))
                                        .rounding(4.0)
                                        .inner_margin(Vec2::new(8.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.colored_label(
                                                theme.text_gutter,
                                                RichText::new("○ NOT DETECTED")
                                                    .strong()
                                                    .size(10.5),
                                            );
                                        });
                                }
                            });
                        });

                        ui.add_space(4.0);

                        // Description + Latest Activity Info
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tool.description()).color(theme.text_muted).size(11.5));

                            if let Some(s) = latest_session {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("Latest: {}", s.short_timestamp()))
                                            .monospace()
                                            .color(theme.text_muted)
                                            .size(11.0),
                                    );
                                });
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Metrics Bottom Bar
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(14.0, 0.0);

                            // Sessions
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("SESSIONS:").size(10.5).color(theme.text_muted));
                                ui.label(RichText::new(session_count.to_string()).strong().size(12.0).color(theme.text_primary));
                            });

                            // Changes
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("CHANGES:").size(10.5).color(theme.text_muted));
                                ui.label(RichText::new(total_changes.to_string()).strong().size(12.0).color(theme.text_primary));
                            });

                            // Lines Added
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ADDED:").size(10.5).color(theme.text_muted));
                                ui.colored_label(theme.diff_add_text, RichText::new(format!("+{}", lines_added)).strong().size(12.0));
                            });

                            // Lines Deleted
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("DELETED:").size(10.5).color(theme.text_muted));
                                ui.colored_label(theme.diff_del_text, RichText::new(format!("-{}", lines_deleted)).strong().size(12.0));
                            });

                            // Net LOC
                            let net_loc = format_net_loc(lines_added, lines_deleted);
                            let net_color = if lines_added >= lines_deleted { theme.diff_add_text } else { theme.diff_del_text };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("NET:").size(10.5).color(theme.text_muted));
                                ui.colored_label(net_color, RichText::new(net_loc).strong().size(12.0));
                            });

                            // Action Link / Button on right
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn_label = if session_count > 0 {
                                    format!("Explore {} Sessions ➔", session_count)
                                } else {
                                    "Open Sessions ➔".to_string()
                                };

                                let btn = egui::Button::new(
                                    RichText::new(btn_label)
                                        .size(11.5)
                                        .strong()
                                        .color(tool_color),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::new(1.0_f32, tool_color))
                                .rounding(4.0);

                                if ui.add(btn).clicked() {
                                    selected_agent = Some(tool);
                                }
                            });
                        });
                    });

                    let is_hovered = card_resp.response.hovered();
                    if is_hovered {
                        ui.painter().rect_stroke(card_resp.response.rect, 6.0, Stroke::new(1.5_f32, tool_color));
                    }

                    if card_resp.response.interact(egui::Sense::click()).clicked() {
                        selected_agent = Some(tool);
                    }

                    ui.add_space(8.0);
                }

                ui.add_space(16.0);
            });

        selected_agent
    }
}

fn metric_stat(ui: &mut Ui, label: &str, value: &str, color: Color32, muted_color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(10.0).strong().color(muted_color));
        ui.label(RichText::new(value).size(18.0).strong().color(color));
    });
}
