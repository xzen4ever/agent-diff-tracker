use egui::{Color32, RichText, ScrollArea, Stroke, TextEdit, Ui, Vec2};

use crate::models::{format_net_loc, ChatSession, ToolType};
use crate::ui::theme::{Theme, ThemeMode};

pub enum TimelineAction {
    BackToOverview,
    SelectSession(String),
}

pub struct SessionTimelineView;

impl SessionTimelineView {
    pub fn show(
        ui: &mut Ui,
        sessions: &[ChatSession],
        tool: ToolType,
        search_query: &mut String,
        theme: &Theme,
    ) -> Option<TimelineAction> {
        let mut action = None;

        // Filter sessions belonging to this tool
        let tool_sessions: Vec<&ChatSession> = sessions.iter().filter(|s| s.tool == tool).collect();
        let query_lower = search_query.trim().to_lowercase();
        let filtered_sessions: Vec<&ChatSession> = tool_sessions
            .iter()
            .copied()
            .filter(|s| s.matches_query(&query_lower))
            .collect();

        let tool_rgb = tool.color_for_mode(theme.mode == ThemeMode::Light);
        let tool_color = Color32::from_rgb(tool_rgb[0], tool_rgb[1], tool_rgb[2]);

        let total_added: usize = tool_sessions.iter().map(|s| s.total_lines_added).sum();
        let total_deleted: usize = tool_sessions.iter().map(|s| s.total_lines_deleted).sum();
        let total_changes: usize = tool_sessions.iter().map(|s| s.changes.len()).sum();

        ui.vertical(|ui| {
            // Header: Breadcrumb & Agent Stats
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Back Button
                let back_btn = egui::Button::new(
                    RichText::new("⮜ All Agents")
                        .size(12.0)
                        .strong()
                        .color(theme.accent_blue),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0_f32, theme.border_card))
                .rounding(4.0);

                if ui.add(back_btn).clicked() {
                    action = Some(TimelineAction::BackToOverview);
                }

                ui.label(RichText::new("›").size(14.0).color(theme.text_gutter));

                // Tool Badge & Name
                let badge_bg = Color32::from_rgba_premultiplied(tool_rgb[0], tool_rgb[1], tool_rgb[2], 25);
                egui::Frame::none()
                    .fill(badge_bg)
                    .rounding(4.0)
                    .inner_margin(Vec2::new(8.0, 3.0))
                    .show(ui, |ui| {
                        ui.colored_label(
                            tool_color,
                            RichText::new(format!("{} {}", tool.icon(), tool.display_name()))
                                .strong()
                                .size(13.0),
                        );
                    });

                // Right metrics pill
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let net_loc = format_net_loc(total_added, total_deleted);
                    let net_color = if total_added >= total_deleted { theme.diff_add_text } else { theme.diff_del_text };

                    egui::Frame::none()
                        .fill(theme.bg_panel)
                        .stroke(Stroke::new(1.0_f32, theme.border_subtle))
                        .rounding(4.0)
                        .inner_margin(Vec2::new(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                            ui.label(RichText::new(format!("{} Sessions", tool_sessions.len())).strong().size(11.0).color(theme.text_primary));
                            ui.label(RichText::new("│").color(theme.border_subtle));
                            ui.label(RichText::new(format!("{} Changes", total_changes)).size(11.0).color(theme.text_muted));
                            ui.label(RichText::new("│").color(theme.border_subtle));
                            ui.colored_label(theme.diff_add_text, RichText::new(format!("+{}", total_added)).strong().size(11.0));
                            ui.colored_label(theme.diff_del_text, RichText::new(format!("-{}", total_deleted)).strong().size(11.0));
                            ui.label(RichText::new("│").color(theme.border_subtle));
                            ui.colored_label(net_color, RichText::new(format!("Net: {}", net_loc)).strong().size(11.0));
                        });
                });
            });

            ui.add_space(8.0);

            // Filter Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new("⌕").size(14.0).strong().color(theme.text_muted));

                let search_box = egui::Frame::none()
                    .fill(theme.bg_code)
                    .stroke(Stroke::new(1.0_f32, theme.border_subtle))
                    .rounding(4.0)
                    .inner_margin(Vec2::new(8.0, 5.0));

                search_box.show(ui, |ui| {
                    ui.add(
                        TextEdit::singleline(search_query)
                            .hint_text("Filter sessions by prompt, task, title, or file name...")
                            .text_color(theme.text_primary)
                            .desired_width(ui.available_width() - 240.0),
                    );
                });

                if !search_query.is_empty() && ui.button(RichText::new("Clear").size(11.0)).clicked() {
                    search_query.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Showing {} of {} sessions (Newest first)",
                            filtered_sessions.len(),
                            tool_sessions.len()
                        ))
                        .size(11.0)
                        .color(theme.text_muted),
                    );
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Sessions List Area
            if filtered_sessions.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("⌕").size(32.0).color(theme.text_muted));
                        ui.add_space(8.0);
                        if tool_sessions.is_empty() {
                            ui.heading(
                                RichText::new(format!("No sessions recorded for {}", tool.display_name()))
                                    .color(theme.text_primary)
                                    .size(15.0),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Run a coding session with this assistant to see recorded code modifications here.")
                                    .color(theme.text_muted)
                                    .size(12.0),
                            );
                        } else {
                            ui.heading(RichText::new("No matching sessions found").color(theme.text_primary).size(15.0));
                            ui.add_space(4.0);
                            ui.label(RichText::new("Try changing your search query.").color(theme.text_muted).size(12.0));
                            ui.add_space(8.0);
                            if ui.button("Clear Search").clicked() {
                                search_query.clear();
                            }
                        }
                    });
                });
            } else {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);

                        for session in &filtered_sessions {
                            let row_frame = egui::Frame::none()
                                .fill(theme.bg_card)
                                .stroke(Stroke::new(1.0_f32, theme.border_card))
                                .rounding(6.0)
                                .inner_margin(egui::Margin::symmetric(12.0, 8.0));

                            let row_resp = row_frame.show(ui, |ui| {
                                // Top Row: Timestamp + Short ID + LOC stats
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                                    // Timestamp
                                    ui.label(
                                        RichText::new(session.short_timestamp())
                                            .monospace()
                                            .strong()
                                            .size(11.5)
                                            .color(theme.text_primary),
                                    );

                                    // Short ID badge
                                    egui::Frame::none()
                                        .fill(theme.bg_code)
                                        .rounding(4.0)
                                        .inner_margin(Vec2::new(5.0, 1.0))
                                        .show(ui, |ui| {
                                            ui.colored_label(
                                                theme.text_gutter,
                                                RichText::new(format!("#{}", session.short_id()))
                                                    .monospace()
                                                    .size(10.5),
                                            );
                                        });

                                    // Right LOC and file stats
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.colored_label(
                                            tool_color,
                                            RichText::new("➔").strong().size(13.0),
                                        );

                                        if session.total_lines_deleted > 0 {
                                            ui.colored_label(
                                                theme.diff_del_text,
                                                RichText::new(format!("-{}", session.total_lines_deleted))
                                                    .strong()
                                                    .size(11.5),
                                            );
                                        }
                                        if session.total_lines_added > 0 {
                                            ui.colored_label(
                                                theme.diff_add_text,
                                                RichText::new(format!("+{}", session.total_lines_added))
                                                    .strong()
                                                    .size(11.5),
                                            );
                                        }

                                        ui.label(RichText::new("│").color(theme.border_subtle));

                                        ui.label(
                                            RichText::new(format!("{} changes", session.changes.len()))
                                                .size(11.0)
                                                .color(theme.text_muted),
                                        );

                                        ui.label(RichText::new("│").color(theme.border_subtle));

                                        // Count unique modified files in this session
                                        let unique_session_files = {
                                            let mut set = std::collections::HashSet::new();
                                            for c in &session.changes {
                                                set.insert(&c.file_path);
                                            }
                                            set.len()
                                        };
                                        ui.label(
                                            RichText::new(format!("{} files", unique_session_files))
                                                .size(11.0)
                                                .color(theme.text_muted),
                                        );
                                    });
                                });

                                ui.add_space(4.0);

                                // Main Row: Session Title / Prompt
                                ui.horizontal(|ui| {
                                    let title_text = if session.title.trim().is_empty() {
                                        "Untitled Session"
                                    } else {
                                        session.title.trim()
                                    };

                                    let display_title = if title_text.len() > 110 {
                                        format!("{}...", &title_text[..107])
                                    } else {
                                        title_text.to_string()
                                    };

                                    ui.label(
                                        RichText::new(display_title)
                                            .strong()
                                            .size(12.5)
                                            .color(theme.text_primary),
                                    );
                                });

                                ui.add_space(2.0);

                                // File Path previews
                                ui.horizontal(|ui| {
                                    let files: Vec<&str> = session
                                        .changes
                                        .iter()
                                        .map(|c| {
                                            std::path::Path::new(&c.file_path)
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or(&c.file_path)
                                        })
                                        .take(4)
                                        .collect();

                                    if !files.is_empty() {
                                        ui.label(RichText::new("Files:").color(theme.text_gutter).size(10.5));
                                        for f in files {
                                            egui::Frame::none()
                                                .fill(theme.bg_code)
                                                .rounding(3.0)
                                                .inner_margin(Vec2::new(4.0, 1.0))
                                                .show(ui, |ui| {
                                                ui.colored_label(
                                                    theme.text_muted,
                                                    RichText::new(f).monospace().size(10.0),
                                                );
                                            });
                                        }
                                        if session.changes.len() > 4 {
                                            ui.label(
                                                RichText::new(format!("+{} more", session.changes.len() - 4))
                                                    .color(theme.text_gutter)
                                                    .size(10.0),
                                            );
                                        }
                                    }
                                });
                            });

                            let is_hovered = row_resp.response.hovered();
                            if is_hovered {
                                ui.painter().rect_stroke(row_resp.response.rect, 6.0, Stroke::new(1.0_f32, tool_color));
                            }

                            if row_resp.response.interact(egui::Sense::click()).clicked() {
                                action = Some(TimelineAction::SelectSession(session.id.clone()));
                            }
                        }

                        ui.add_space(10.0);
                    });
            }
        });

        action
    }
}
