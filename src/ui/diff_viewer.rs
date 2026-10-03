use egui::{Color32, FontId, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::{CodeChange, DiffLineType};
use crate::ui::theme::{Theme, ThemeMode};

pub struct DiffViewer;

impl DiffViewer {
    pub fn show(ui: &mut Ui, change: &CodeChange, theme: &Theme) {
        ui.vertical(|ui| {
            // Header bar for the selected file change
            let header_frame = egui::Frame::none()
                .fill(theme.bg_panel)
                .stroke(Stroke::new(1.0_f32, theme.border_subtle))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0));

            header_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let tool_rgb = change.tool.color_for_mode(theme.mode == ThemeMode::Light);
                    let tool_color = Color32::from_rgb(tool_rgb[0], tool_rgb[1], tool_rgb[2]);
                    let tool_bg = Color32::from_rgba_premultiplied(tool_rgb[0], tool_rgb[1], tool_rgb[2], 25);

                    // Tool badge
                    egui::Frame::none()
                        .fill(tool_bg)
                        .rounding(4.0)
                        .inner_margin(Vec2::new(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.colored_label(
                                tool_color,
                                RichText::new(change.tool.display_name()).strong().size(11.0),
                            );
                        });

                    // Change Type Badge
                    let type_bg = match change.change_type {
                        crate::models::ChangeType::Create => theme.diff_add_bg,
                        crate::models::ChangeType::Delete => theme.diff_del_bg,
                        crate::models::ChangeType::Modify => theme.diff_hdr_bg,
                    };
                    let type_fg = match change.change_type {
                        crate::models::ChangeType::Create => theme.diff_add_text,
                        crate::models::ChangeType::Delete => theme.diff_del_text,
                        crate::models::ChangeType::Modify => theme.diff_hdr_text,
                    };

                    egui::Frame::none()
                        .fill(type_bg)
                        .rounding(4.0)
                        .inner_margin(Vec2::new(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.colored_label(
                                type_fg,
                                RichText::new(change.change_type.badge()).strong().size(11.0),
                            );
                        });

                    // Timestamp
                    ui.label(RichText::new(&change.timestamp).monospace().color(theme.text_muted).size(11.0));

                    // LOC Delta
                    ui.colored_label(
                        theme.diff_add_text,
                        RichText::new(format!("+{}", change.lines_added)).strong().size(12.0),
                    );
                    ui.colored_label(
                        theme.diff_del_text,
                        RichText::new(format!("-{}", change.lines_deleted)).strong().size(12.0),
                    );

                    // Right Actions Toolbar
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("⎘ Copy Diff").size(11.0)).clicked() {
                            let mut full_diff = format!("--- {}\n+++ {}\n", change.file_path, change.file_path);
                            for l in &change.diff_lines {
                                full_diff.push_str(&l.text);
                                full_diff.push('\n');
                            }
                            ui.output_mut(|o| o.copied_text = full_diff);
                        }

                        if ui.button(RichText::new("↗ Open Folder").size(11.0)).clicked() {
                            if let Some(parent) = std::path::Path::new(&change.file_path).parent() {
                                if parent.exists() {
                                    #[cfg(target_os = "windows")]
                                    {
                                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                                    }
                                    #[cfg(not(target_os = "windows"))]
                                    {
                                        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                                    }
                                }
                            }
                        }

                        if ui.button(RichText::new("⎘ Copy Path").size(11.0)).clicked() {
                            ui.output_mut(|o| o.copied_text = change.file_path.clone());
                        }
                    });
                });

                ui.add_space(4.0);

                // File path display
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Path:").color(theme.text_muted).size(11.5));
                    ui.colored_label(
                        theme.text_primary,
                        RichText::new(&change.file_path).monospace().strong().size(12.0),
                    );
                });

                // Description and Action
                if !change.description.is_empty() || !change.session_title.is_empty() {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        if !change.description.is_empty() {
                            ui.label(RichText::new("Action:").color(theme.text_muted).size(11.0));
                            ui.label(RichText::new(&change.description).size(11.5).color(theme.text_primary));
                        }

                        if !change.session_title.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Session:").color(theme.text_muted).size(11.0));
                            ui.label(RichText::new(&change.session_title).italics().size(11.5).color(theme.text_muted));
                        }
                    });
                }
            });

            ui.add_space(4.0);

            // Diff Code Canvas
            ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    let code_canvas = egui::Frame::canvas(ui.style())
                        .fill(theme.bg_code)
                        .stroke(Stroke::new(1.0_f32, theme.border_subtle))
                        .rounding(6.0)
                        .inner_margin(8.0);

                    code_canvas.show(ui, |ui| {
                        let mono_font = FontId::monospace(13.0);

                        for line in &change.diff_lines {
                            let (bg_color, text_color, prefix) = match line.line_type {
                                DiffLineType::Added => (theme.diff_add_bg, theme.diff_add_text, "+ "),
                                DiffLineType::Removed => (theme.diff_del_bg, theme.diff_del_text, "- "),
                                DiffLineType::Header => (theme.diff_hdr_bg, theme.diff_hdr_text, "@@"),
                                DiffLineType::Context => (Color32::TRANSPARENT, theme.text_primary, "  "),
                            };

                            let old_num_str = line
                                .old_line_num
                                .map(|n| format!("{:>4}", n))
                                .unwrap_or_else(|| "    ".to_string());
                            let new_num_str = line
                                .new_line_num
                                .map(|n| format!("{:>4}", n))
                                .unwrap_or_else(|| "    ".to_string());

                            let line_frame = egui::Frame::none()
                                .fill(bg_color)
                                .inner_margin(egui::Margin::symmetric(2.0, 1.0));

                            line_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                    // Line numbers (Gutter)
                                    ui.colored_label(
                                        theme.text_gutter,
                                        RichText::new(&old_num_str).monospace().font(mono_font.clone()),
                                    );
                                    ui.colored_label(
                                        theme.text_gutter,
                                        RichText::new(&new_num_str).monospace().font(mono_font.clone()),
                                    );

                                    // Gutter separator hairline
                                    ui.colored_label(
                                        theme.border_subtle,
                                        RichText::new("│").font(mono_font.clone()),
                                    );

                                    // Prefix indicator (+, -, @@, space)
                                    ui.colored_label(
                                        text_color,
                                        RichText::new(prefix).monospace().font(mono_font.clone()).strong(),
                                    );

                                    // Line text
                                    ui.colored_label(
                                        text_color,
                                        RichText::new(&line.text).monospace().font(mono_font.clone()),
                                    );
                                });
                            });
                        }
                    });
                });
        });
    }
}
