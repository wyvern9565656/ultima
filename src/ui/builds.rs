use egui::{Frame, Margin, RichText, Stroke, Ui};

use crate::app::App;
use crate::theme::{self, ACCENT, AMBER, BORDER, CARD, DANGER, FAINT, LOG_BG, MUTED, TEXT};
use crate::worker::LogLevel;

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    if app.jobs.is_empty() {
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Nenhuma compilação ainda").size(15.0).color(MUTED));
            ui.add_space(6.0);
            ui.label(
                RichText::new("Escolha um app na Biblioteca e use Compilar do código.")
                    .size(12.5)
                    .color(FAINT),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("builds-scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 14.0;
            for job in app.jobs.iter().rev() {
                Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(theme::r(12))
                    .inner_margin(Margin::symmetric(16, 14))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&job.title).size(15.0).strong().color(TEXT));
                            ui.add_space(6.0);
                            let (label, color) = match &job.done {
                                Some(Ok(())) => ("concluída".to_string(), ACCENT),
                                Some(Err(err)) => (format!("falhou · {err}"), DANGER),
                                None => (format!("em andamento · {:.0}s", job.elapsed()), AMBER),
                            };
                            ui.label(RichText::new(label).size(12.0).color(color));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(&job.repo)
                                        .size(11.5)
                                        .color(FAINT),
                                );
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "$ {} · {}",
                                job.command,
                                match job.done {
                                    Some(_) => "finalizado".to_string(),
                                    None => job.phase.clone(),
                                }
                            ))
                            .monospace()
                            .size(11.5)
                            .color(MUTED),
                        );
                        ui.add_space(8.0);

                        Frame::new()
                            .fill(LOG_BG)
                            .corner_radius(theme::r(8))
                            .inner_margin(Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(220.0)
                                    .auto_shrink([false, false])
                                    .stick_to_bottom(true)
                                    .id_salt(("build-log", job.id))
                                    .show(ui, |ui| {
                                        if job.log.is_empty() {
                                            ui.label(
                                                RichText::new("aguardando saída…")
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(FAINT),
                                            );
                                        }
                                        for (level, line) in &job.log {
                                            let color = match level {
                                                LogLevel::Info => {
                                                    egui::Color32::from_rgb(0xC3, 0xD3, 0xD9)
                                                }
                                                LogLevel::Warn => AMBER,
                                                LogLevel::Error => DANGER,
                                            };
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(line)
                                                        .monospace()
                                                        .size(11.0)
                                                        .color(color),
                                                )
                                                .wrap(),
                                            );
                                        }
                                    });
                            });
                    });
            }
        });
}
