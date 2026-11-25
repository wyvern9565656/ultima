use egui::{Align, Align2, Frame, Layout, Margin, RichText, Sense, Vec2};

use crate::app::{App, Page};
use crate::theme::{self, ACCENT, TEXT};

pub fn show(app: &mut App, root: &mut egui::Ui) {
    let frame = Frame::new().fill(theme::SIDEBAR).inner_margin(Margin {
        left: 20,
        right: 20,
        top: 26,
        bottom: 22,
    });
    egui::Panel::left("ultima-sidebar")
        .exact_size(296.0)
        .resizable(false)
        .frame(frame)
        .show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("U").size(30.0).strong().color(ACCENT));
                ui.add_space(6.0);
                ui.label(RichText::new("ULTIMA").size(18.0).strong().color(TEXT));
            });

            ui.add_space(26.0);
            if theme::nav_row(ui, "Biblioteca", app.page == Page::Library, None).clicked() {
                app.goto(Page::Library);
            }
            if theme::nav_row(ui, "Descobrir", app.page == Page::Discover, None).clicked() {
                app.goto(Page::Discover);
            }
            let updates = app.updates_count();
            if theme::nav_row(ui, "Atualizações", app.page == Page::Updates, Some(updates)).clicked() {
                app.goto(Page::Updates);
            }
            if theme::nav_row(ui, "Compilações", app.page == Page::Builds, Some(app.active_jobs())).clicked() {
                app.goto(Page::Builds);
            }

            ui.add_space(22.0);
            theme::section_label(ui, "Famílias");
            ui.add_space(6.0);

            let families = app.families();
            for (family, count) in families {
                let first = app
                    .config
                    .repos
                    .iter()
                    .find(|r| (if r.family.is_empty() { "Conectados" } else { &r.family }) == family)
                    .map(|r| r.full_name.clone());
                if family_row(ui, &family, count).clicked() {
                    if let Some(id) = first {
                        app.select_repo(&id);
                        app.goto(Page::Library);
                    }
                }
            }

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                if ui
                    .add_sized(
                        Vec2::new(ui.available_width(), 38.0),
                        theme::button("Configurações", theme::Btn::Secondary),
                    )
                    .clicked()
                {
                    app.token_draft = app.config.token.clone().unwrap_or_default();
                    app.dir_draft = app.config.install_dir.display().to_string();
                    app.settings_error = None;
                    app.show_settings = true;
                }
                ui.add_space(12.0);
                let connected = app.config.repos.len();
                let text = match app.last_check {
                    Some(at) => {
                        let secs = at.elapsed().as_secs();
                        let when = if secs < 60 {
                            format!("{secs}s")
                        } else {
                            format!("{} min", secs / 60)
                        };
                        format!("{connected} repositórios conectados · há {when}")
                    }
                    None => format!("{connected} repositórios conectados"),
                };
                ui.label(RichText::new(text).size(12.0).color(theme::FAINT));
            });
        });
}

fn family_row(ui: &mut egui::Ui, label: &str, count: usize) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(rect, theme::r(7), theme::CARD);
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        theme::MUTED,
    );
    ui.painter().text(
        rect.right_center() - Vec2::new(14.0, 0.0),
        Align2::RIGHT_CENTER,
        count.to_string(),
        egui::FontId::proportional(12.0),
        theme::FAINT,
    );
    response
}
