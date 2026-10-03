use egui::{Color32, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::models::ChatSession;
use crate::ui::diff_viewer::DiffViewer;
use crate::ui::theme::{Theme, ThemeMode};

pub enum DetailAction {
    BackToOverview,
    BackToSessions,
}

pub struct SessionDetailView;

impl SessionDetailView {
    pub fn show(
        ui: &mut Ui,
        session: &ChatSession,
        selected_change_idx: &mut usize,
        theme: &Theme,
    ) -> Option<DetailAction> {
        let mut action = None;

        let tool_rgb = session.tool.color_for_mode(theme.mode == ThemeMode::Light);
        let tool_color = Color32::from_rgb(tool_rgb[0], tool_rgb[1], tool_rgb[2]);

        ui.vertical(|ui| {
            // Header Bar: Hierarchical Breadcrumb Navigation
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Step 1: All Agents
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("⮜ All Agents")
                                .size(11.5)
                                .color(theme.text_muted),
                        )
                        .fill(Color32::TRANSPARENT),
                    )
                    .clicked()
                {
                    action = Some(DetailAction::BackToOverview);
                }

                ui.label(RichText::new("›").size(13.0).color(theme.text_gutter));

                // Step 2: Agent Sessions
                let agent_back_label = format!("⮜ {} Sessions", session.tool.display_name());
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(agent_back_label)
                                .size(11.5)
                                .strong()
                                .color(tool_color),
                        )
                        .fill(Color32::TRANSPARENT),
                    )
                    .clicked()
                {
                    action = Some(DetailAction::BackToSessions);
                }

                ui.label(RichText::new("›").size(13.0).color(theme.text_gutter));

                // Step 3: Current Session
                let session_short_title = if session.title.trim().is_empty() {
                    format!("Session #{}", session.short_id())
                } else if session.title.len() > 60 {
                    format!("{}...", &session.title[..57])
                } else {
                    session.title.clone()
                };

                ui.label(
                    RichText::new(session_short_title)
                        .strong()
                        .size(12.0)
                        .color(theme.text_primary),
                );
            });

            ui.add_space(6.0);

            // Session Metadata Card Strip
            let meta_frame = egui::Frame::none()
                .fill(theme.bg_panel)
                .stroke(Stroke::new(1.0_f32, theme.border_card))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0));

            meta_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Time:").color(theme.text_muted).size(11.0));
                    ui.label(RichText::new(&session.timestamp).monospace().color(theme.text_primary).size(11.5));

                    ui.label(RichText::new("│").color(theme.border_subtle));

                    ui.label(RichText::new("ID:").color(theme.text_muted).size(11.0));
                    ui.label(
                        RichText::new(&session.id)
                            .monospace()
                            .color(theme.text_primary)
                            .size(11.0),
                    );

                    ui.label(RichText::new("│").color(theme.border_subtle));

                    ui.label(
                        RichText::new(format!("{} modified files", session.changes.len()))
                            .size(11.5)
                            .color(theme.text_primary),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if session.total_lines_deleted > 0 {
                            ui.colored_label(
                                theme.diff_del_text,
                                RichText::new(format!("-{}", session.total_lines_deleted))
                                    .strong()
                                    .size(12.0),
                            );
                        }
                        if session.total_lines_added > 0 {
                            ui.colored_label(
                                theme.diff_add_text,
                                RichText::new(format!("+{}", session.total_lines_added))
                                    .strong()
                                    .size(12.0),
                            );
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Main Content: Left file list + Right diff viewer
            if session.changes.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("≡").size(32.0).color(theme.text_muted));
                        ui.add_space(8.0);
                        ui.heading(RichText::new("No code changes recorded for this session").color(theme.text_primary).size(15.0));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("This session might have only contained chat responses without file edits.")
                                .color(theme.text_muted)
                                .size(12.0),
                        );
                    });
                });
            } else {
                if *selected_change_idx >= session.changes.len() {
                    *selected_change_idx = 0;
                }

                ui.horizontal(|ui| {
                    // Left Column: Modified Files List
                    let files_frame = egui::Frame::none()
                        .fill(theme.bg_panel)
                        .stroke(Stroke::new(1.0_f32, theme.border_card))
                        .rounding(6.0)
                        .inner_margin(8.0);

                    files_frame.show(ui, |ui| {
                        ui.set_width(290.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("MODIFIED FILES ({})", session.changes.len()))
                                        .strong()
                                        .size(11.0)
                                        .color(theme.text_muted),
                                );
                            });

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 3.0);

                                    for (idx, change) in session.changes.iter().enumerate() {
                                        let is_selected = idx == *selected_change_idx;
                                        let filename = std::path::Path::new(&change.file_path)
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| change.file_path.clone());

                                        let row_frame = egui::Frame::none()
                                            .fill(if is_selected { theme.bg_selected } else { Color32::TRANSPARENT })
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin {
                                                left: 8.0,
                                                right: 6.0,
                                                top: 6.0,
                                                bottom: 6.0,
                                            });

                                        let row_resp = row_frame.show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    let display_name = if filename.len() > 22 {
                                                        format!("{}...", &filename[..19])
                                                    } else {
                                                        filename.clone()
                                                    };

                                                    ui.colored_label(
                                                        theme.text_primary,
                                                        RichText::new(display_name).monospace().strong().size(12.0),
                                                    );

                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if change.lines_deleted > 0 {
                                                            ui.colored_label(
                                                                theme.diff_del_text,
                                                                RichText::new(format!("-{}", change.lines_deleted)).size(10.5),
                                                            );
                                                        }
                                                        if change.lines_added > 0 {
                                                            ui.colored_label(
                                                                theme.diff_add_text,
                                                                RichText::new(format!("+{}", change.lines_added)).size(10.5),
                                                            );
                                                        }
                                                    });
                                                });

                                                // Subtitle: Action or directory path
                                                let parent_path = std::path::Path::new(&change.file_path)
                                                    .parent()
                                                    .map(|p| p.to_string_lossy().to_string())
                                                    .unwrap_or_default();

                                                if !parent_path.is_empty() {
                                                    let short_path = if parent_path.len() > 34 {
                                                        format!("...{}", &parent_path[parent_path.len() - 31..])
                                                    } else {
                                                        parent_path
                                                    };
                                                    ui.label(
                                                        RichText::new(short_path)
                                                            .color(theme.text_muted)
                                                            .size(10.0),
                                                    );
                                                } else if !change.description.is_empty() {
                                                    ui.label(
                                                        RichText::new(&change.description)
                                                            .color(theme.text_muted)
                                                            .size(10.0),
                                                    );
                                                }
                                            });
                                        });

                                        // Active Indicator Bar on Left
                                        if is_selected {
                                            let rect = row_resp.response.rect;
                                            let accent_rect = egui::Rect::from_min_size(
                                                rect.left_top(),
                                                Vec2::new(3.0, rect.height()),
                                            );
                                            ui.painter().rect_filled(accent_rect, 1.0, theme.accent_blue);
                                        }

                                        if row_resp.response.interact(egui::Sense::click()).clicked() {
                                            *selected_change_idx = idx;
                                        }
                                    }
                                });
                        });
                    });

                    ui.add_space(6.0);

                    // Right Column: Diff Inspector Canvas
                    let current_change = &session.changes[*selected_change_idx];
                    DiffViewer::show(ui, current_change, theme);
                });
            }
        });

        action
    }
}
