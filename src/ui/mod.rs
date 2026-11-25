pub mod builds;
pub mod details;
pub mod discover;
pub mod library;
pub mod settings;
pub mod sidebar;

use egui::{Align2, Frame, Margin, Order, RichText, Stroke, Vec2};

use crate::app::{App, Filter, Page};
use crate::theme::{self, Btn, BORDER, FIELD, MUTED, TEXT};
use crate::worker::LogLevel;

pub fn central(app: &mut App, root: &mut egui::Ui) {
    let frame = Frame::new()
        .fill(theme::BG)
        .inner_margin(Margin {
            left: 26,
            right: 26,
            top: 20,
            bottom: 18,
        });
    egui::CentralPanel::default().frame(frame).show(root, |ui| {
        header(app, ui);
        ui.add_space(6.0);
        match app.page {
            Page::Library | Page::Updates => library::show(app, ui),
            Page::Discover => discover::show(app, ui),
            Page::Builds => builds::show(app, ui),
        }
    });
}

fn header(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let total = ui.available_width();
        let button_w = 212.0;
        let field_w = (total - button_w - 14.0).max(220.0);
        search_field(ui, app, field_w);
        if ui
            .add_sized(
                Vec2::new(button_w, 40.0),
                theme::button("Verificar atualizações", Btn::Secondary),
            )
            .clicked()
        {
            app.refresh_all();
        }
        if !app.loading.is_empty() {
            ui.add(egui::Spinner::new().size(14.0));
        }
    });

    ui.add_space(14.0);

    ui.horizontal(|ui| {
        let title = match app.page {
            Page::Library => "Biblioteca",
            Page::Discover => "Descobrir",
            Page::Updates => "Atualizações",
            Page::Builds => "Compilações",
        };
        ui.label(RichText::new(title).size(26.0).strong().color(TEXT));
        ui.add_space(14.0);

        if matches!(app.page, Page::Library | Page::Updates) {
            ui.spacing_mut().item_spacing.x = 8.0;
            if app.page == Page::Library {
                let counts = (
                    app.config.repos.len(),
                    app.installs.len(),
                    app.updates_count(),
                    app.repos_source_only(),
                );
                let chips = [
                    ("Todos", Filter::All, counts.0),
                    ("Instalados", Filter::Installed, counts.1),
                    ("Com atualização", Filter::Updates, counts.2),
                    ("Só código", Filter::SourceOnly, counts.3),
                ];
                for (label, filter, count) in chips {
                    let text = if count > 0 {
                        format!("{label} · {count}")
                    } else {
                        label.to_string()
                    };
                    if theme::chip(ui, &text, app.filter == filter).clicked() {
                        app.filter = filter;
                    }
                }
            }
        }
    });
    ui.add_space(8.0);
}

fn search_field(ui: &mut egui::Ui, app: &mut App, width: f32) {
    Frame::new()
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(theme::r(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.add_sized(
                [width - 28.0, 20.0],
                egui::TextEdit::singleline(&mut app.search)
                    .hint_text(RichText::new("Buscar nos seus repositórios…").color(MUTED))
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });
}

pub fn toasts(app: &App, ctx: &egui::Context) {
    if app.toasts.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::unique("ultima-toasts"))
        .order(Order::Foreground)
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-28.0, -26.0))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                for toast in &app.toasts {
                    let accent = match toast.level {
                        LogLevel::Info => theme::ACCENT,
                        LogLevel::Warn => theme::AMBER,
                        LogLevel::Error => theme::DANGER,
                    };
                    Frame::new()
                        .fill(theme::CARD)
                        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
                        .corner_radius(theme::r(10))
                        .inner_margin(Margin::symmetric(14, 11))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                theme::dot(ui, accent, 8.0);
                                ui.label(RichText::new(&toast.text).size(13.0).color(TEXT));
                            });
                        });
                    ui.add_space(8.0);
                }
            });
        });
}
