use egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2};

use crate::app::App;
use crate::theme::{self, Btn, ACCENT, BORDER, CARD, DANGER, MUTED, TEXT};

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);

    if app.discover.is_none() && !app.discover_loading {
        app.load_discover();
    }

    if let Some(err) = &app.discover_error {
        ui.add_space(8.0);
        ui.label(RichText::new(err).size(13.0).color(DANGER));
        ui.add_space(6.0);
        ui.label(
            RichText::new("Dica: configure um token do GitHub em Configurações.")
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(10.0);
        if ui.add(theme::button("Tentar novamente", Btn::Secondary)).clicked() {
            app.discover = None;
            app.discover_error = None;
            app.load_discover();
        }
        return;
    }

    let Some(repos) = app.discover.clone() else {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Spinner::new().size(20.0));
            ui.add_space(8.0);
            ui.label(RichText::new("Consultando seus repositórios…").size(13.0).color(MUTED));
        });
        return;
    };

    ui.label(
        RichText::new(format!(
            "{} repositórios na sua conta · conecte os que quiser acompanhar aqui",
            repos.len()
        ))
        .size(12.5)
        .color(MUTED),
    );
    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("discover-scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            for info in repos {
                let connected = app
                    .config
                    .repos
                    .iter()
                    .any(|r| r.full_name.eq_ignore_ascii_case(&info.full_name));
                Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(theme::r(11))
                    .inner_margin(Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            theme::icon_tile(
                                ui,
                                38.0,
                                crate::model::accent_for(&info.full_name),
                                theme::glyph_for(&info.full_name),
                            );
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&info.full_name).size(14.0).strong().color(TEXT),
                                );
                                let desc = info
                                    .description
                                    .clone()
                                    .unwrap_or_else(|| "sem descrição".into());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(desc).size(12.0).color(MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if connected {
                                    ui.label(
                                        RichText::new("Conectado")
                                            .size(12.0)
                                            .color(ACCENT),
                                    );
                                } else if ui
                                    .add(
                                        theme::button("Conectar", Btn::Secondary)
                                            .min_size(Vec2::new(92.0, 32.0)),
                                    )
                                    .clicked()
                                {
                                    app.add_repo(&info.full_name);
                                }
                                ui.add_space(6.0);
                                if let Some(language) = &info.language {
                                    ui.label(RichText::new(language).size(11.5).color(theme::FAINT));
                                }
                            });
                        });
                    });
            }
        });
}
