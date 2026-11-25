use egui::{Align, Align2, Frame, Layout, Margin, RichText, Stroke, Vec2};

use crate::app::App;
use crate::theme::{self, Btn, ACCENT, BORDER, BORDER_STRONG, CARD, DANGER, MUTED, PANEL, TEXT};

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.show_settings {
        return;
    }
    let mut open = true;
    let mut close = false;

    egui::Window::new("Configurações")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER_STRONG))
                .corner_radius(theme::r(14))
                .inner_margin(Margin::same(22)),
        )
        .show(ctx, |ui| {
            ui.set_width(540.0);

            theme::section_label(ui, "GitHub");
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Um token pessoal aumenta o limite de 60 para 5.000 requisições por hora e dá acesso a repositórios privados.",
                )
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(8.0);
            Frame::new()
                .fill(theme::FIELD)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(theme::r(9))
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 20.0],
                        egui::TextEdit::singleline(&mut app.token_draft)
                            .hint_text("ghp_… (opcional)")
                            .password(true)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                });

            ui.add_space(20.0);
            theme::section_label(ui, "Pasta de instalação");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(theme::FIELD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(theme::r(9))
                    .inner_margin(Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.add_sized(
                            [ui.available_width() - 130.0, 20.0],
                            egui::TextEdit::singleline(&mut app.dir_draft)
                                .hint_text("~/.local/share/ultima")
                                .frame(egui::Frame::NONE)
                                .desired_width(f32::INFINITY),
                        );
                    });
                if ui
                    .add(theme::button("Usar padrão", Btn::Ghost))
                    .clicked()
                {
                    app.dir_draft = crate::model::default_install_dir().display().to_string();
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("Os apps instalados ficam em apps/ e os fontes clonados em src/.")
                    .size(11.5)
                    .color(theme::FAINT),
            );

            ui.add_space(20.0);
            theme::section_label(ui, "Repositórios conectados");
            ui.add_space(8.0);
            let repos = app.config.repos.clone();
            for repo in repos {
                Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(theme::r(9))
                    .inner_margin(Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            theme::dot(ui, crate::model::accent_for(&repo.full_name), 8.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&repo.full_name).size(13.0).color(TEXT));
                                let family = if repo.family.is_empty() {
                                    "Conectados".to_string()
                                } else {
                                    repo.family.clone()
                                };
                                ui.label(
                                    RichText::new(format!("família {family}"))
                                        .size(11.0)
                                        .color(MUTED),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        theme::button("Remover", Btn::Ghost)
                                            .min_size(Vec2::new(80.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    app.remove_repo(&repo.full_name);
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }

            ui.add_space(10.0);
            Frame::new()
                .fill(theme::FIELD)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(theme::r(9))
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let width = ui.available_width() - 108.0;
                        ui.add_sized(
                            [width, 20.0],
                            egui::TextEdit::singleline(&mut app.new_repo_draft)
                                .hint_text("dono/repositório ou URL do GitHub")
                                .frame(egui::Frame::NONE)
                                .desired_width(f32::INFINITY),
                        );
                        if ui
                            .add(theme::button("Adicionar", Btn::Secondary))
                            .clicked()
                        {
                            let input = app.new_repo_draft.clone();
                            app.add_repo(&input);
                        }
                    });
                });

            if let Some(err) = &app.settings_error {
                ui.add_space(8.0);
                ui.label(RichText::new(err).size(12.0).color(DANGER));
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui
                    .add(theme::button("Salvar", Btn::Primary).min_size(Vec2::new(120.0, 36.0)))
                    .clicked()
                {
                    app.apply_settings();
                    close = true;
                }
                if ui
                    .add(theme::button("Cancelar", Btn::Secondary).min_size(Vec2::new(110.0, 36.0)))
                    .clicked()
                {
                    close = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(token) = std::env::var("GITHUB_TOKEN").ok() {
                        if !token.is_empty() {
                            ui.label(
                                RichText::new("GITHUB_TOKEN ativo no ambiente")
                                    .size(11.0)
                                    .color(ACCENT),
                            );
                        }
                    }
                });
            });
        });

    if !open || close {
        app.show_settings = false;
    }
}
