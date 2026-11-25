use egui::{Align, Align2, Frame, Layout, Margin, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::app::App;
use crate::model::{display_tag, human_ago, short_date, Channel, InstalledRecord, Release};
use crate::theme::{
    self, Btn, ACCENT, ACCENT_SOFT, AMBER, BORDER, CARD, DANGER, FAINT, LOG_BG,
    MUTED, TEXT,
};
use crate::worker::LogLevel;

pub fn show(app: &mut App, root: &mut egui::Ui) {
    let frame = Frame::new().fill(theme::PANEL).inner_margin(Margin {
        left: 24,
        right: 24,
        top: 24,
        bottom: 18,
    });
    egui::Panel::right("ultima-details")
        .exact_size(438.0)
        .resizable(false)
        .frame(frame)
        .show(root, |ui| {
            let Some(id) = app.selected.clone() else {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Selecione um app").size(15.0).color(MUTED));
                });
                return;
            };
            let Some(repo) = app.repo(&id).cloned() else {
                return;
            };

            header(app, ui, &id, &repo);

            let footer_h = 34.0;
            let upper_h = (ui.available_height() - footer_h).max(120.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), upper_h),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt("details-scroll")
                        .show(ui, |ui| {
                            ui.add_space(16.0);
                            versions(app, ui, &id);
                            ui.add_space(14.0);
                            actions(app, ui, &id);
                            ui.add_space(18.0);
                            job_panel(app, ui, &id);
                        });
                },
            );

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let mut text = format!("Instala em {}", app.install_dir_label());
                    if let Some(record) = app.installs.get(&id) {
                        text.push_str(&format!(" · última em {}", human_ago(record.installed_at)));
                    }
                    ui.label(RichText::new(text).size(11.5).color(FAINT));
                });
            });
        });
}

fn header(app: &App, ui: &mut Ui, id: &str, repo: &crate::model::RepoRef) {
    ui.horizontal(|ui| {
        theme::icon_tile(ui, 52.0, crate::model::accent_for(id), theme::glyph_for(id));
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(RichText::new(repo.title()).size(21.0).strong().color(TEXT));
            let language = app
                .infos
                .get(id)
                .and_then(|i| i.language.clone())
                .unwrap_or_else(|| "código".into());
            let family = app.family_of(id);
            ui.label(
                RichText::new(format!("{language} · família {family}"))
                    .size(12.0)
                    .color(MUTED),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if app.is_busy(id) {
                ui.add(egui::Spinner::new().size(16.0));
            }
        });
    });

    if let Some(err) = app.errors.get(id) {
        ui.add_space(8.0);
        ui.label(RichText::new(err).size(12.0).color(DANGER));
    }

    ui.add_space(22.0);
    theme::section_label(ui, "Versão");
    ui.add_space(6.0);
}

fn versions(app: &mut App, ui: &mut Ui, id: &str) {
    let chosen = app.chosen_version(id);
    let installed = app.installs.get(id).cloned();
    let branch = app.branch_of(id);

    let releases: Vec<Release> = app.releases.get(id).cloned().unwrap_or_default();
    let mut sorted = releases;
    sorted.sort_by(|a, b| crate::model::compare_versions(&b.tag, &a.tag));

    let mut entries: Vec<(String, String, &'static str)> = Vec::new();
    if let Some(record) = &installed {
        if record.channel == Channel::Release
            && !sorted.iter().take(3).any(|r| r.tag == record.version)
        {
            entries.push((record.version.clone(), "instalada".into(), "binário"));
        }
    }
    for release in sorted.iter().take(3) {
        let detail = if release.prerelease {
            "pré-release"
        } else {
            "release estável"
        };
        entries.push((release.tag.clone(), detail.into(), "binário"));
    }

    for (tag, detail, channel) in &entries {
        let label = format!("{} · {}", display_tag(tag), detail);
        let is_installed = installed
            .as_ref()
            .map(|r| r.channel == Channel::Release && r.version == *tag)
            .unwrap_or(false);
        let right = if is_installed { "instalada" } else { channel };
        if version_row(ui, &label, right, chosen == *tag, is_installed).clicked() {
            app.chosen.insert(id.to_string(), tag.clone());
        }
    }

    let head = app.heads.get(id).cloned();
    let head_detail = match &head {
        Some((sha, _)) => format!("{branch} · último commit {sha}"),
        None => format!("{branch} · compilar do código"),
    };
    let head_installed = installed
        .as_ref()
        .map(|r| r.channel == Channel::Source)
        .unwrap_or(false);
    let head_right = if head_installed { "instalada" } else { "compilar" };
    if version_row(ui, &head_detail, head_right, chosen == branch, head_installed).clicked() {
        app.chosen.insert(id.to_string(), branch);
    }

    ui.add_space(4.0);
    if let Some((sha, message)) = head {
        ui.label(
            RichText::new(format!("{sha} · {message}"))
                .size(11.0)
                .color(FAINT),
        );
    }
}

fn version_row(ui: &mut Ui, label: &str, right: &str, selected: bool, installed: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 44.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        theme::r(9),
        if selected { ACCENT_SOFT } else if response.hovered() { CARD } else { theme::PANEL },
    );
    painter.rect_stroke(
        rect,
        theme::r(9),
        Stroke::new(1.0, if selected { ACCENT } else { BORDER }),
        StrokeKind::Inside,
    );

    let center = egui::pos2(rect.left() + 20.0, rect.center().y);
    painter.circle_stroke(center, 6.5, Stroke::new(1.4, if selected { ACCENT } else { MUTED }));
    if selected {
        painter.circle_filled(center, 3.2, ACCENT);
    }

    painter.text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        if selected { TEXT } else { TEXT },
    );

    let right_color = if installed { ACCENT } else { MUTED };
    painter.text(
        egui::pos2(rect.right() - 14.0, rect.center().y),
        Align2::RIGHT_CENTER,
        right,
        egui::FontId::proportional(11.5),
        right_color,
    );
    response
}

fn actions(app: &mut App, ui: &mut Ui, id: &str) {
    let busy = app.is_busy(id);
    let chosen = app.chosen_version(id);
    let branch = app.branch_of(id);
    let installed: Option<InstalledRecord> = app.installs.get(id).cloned();
    let latest = app.latest_stable(id).cloned();
    let chosen_release = app
        .releases
        .get(id)
        .and_then(|list| list.iter().find(|r| r.tag == chosen))
        .cloned();
    let is_source = chosen == branch;

    let installed_matches = installed
        .as_ref()
        .map(|r| match r.channel {
            Channel::Release => r.version == chosen,
            Channel::Source => is_source,
        })
        .unwrap_or(false);

    let width = ui.available_width();
    let primary_w = width * 0.58;
    let secondary_w = width - primary_w - 10.0;

    let mut primary_action: Option<Action> = None;
    let mut secondary_action: Option<Action> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let (label, kind): (String, Btn) = if busy {
            ("Em andamento…".to_string(), Btn::Primary)
        } else if installed_matches {
            ("Abrir".to_string(), Btn::Primary)
        } else if is_source {
            ("Compilar do código".to_string(), Btn::Primary)
        } else if installed.is_some() {
            (format!("Atualizar {}", display_tag(&chosen)), Btn::Amber)
        } else {
            (format!("Instalar {}", display_tag(&chosen)), Btn::Primary)
        };

        let label = label.to_string();
        let button = theme::button(&label, kind).min_size(Vec2::new(primary_w, 40.0));
        let enabled = !busy
            && (is_source || chosen_release.is_some() || installed_matches);
        let response = ui.add_enabled(enabled, button);
        if response.clicked() {
            primary_action = Some(if installed_matches {
                Action::Open
            } else if is_source {
                Action::Compile
            } else {
                Action::Install
            });
        }

        let (second_label, second_enabled, second) = if is_source && installed_matches {
            ("Recompilar".to_string(), true, Some(Action::Compile))
        } else if is_source {
            match &latest {
                Some(release) => (
                    format!("Instalar {}", display_tag(&release.tag)),
                    true,
                    Some(Action::InstallLatest(release.tag.clone())),
                ),
                None => ("Sem releases".to_string(), false, None),
            }
        } else {
            ("Compilar do código".to_string(), true, Some(Action::Compile))
        };

        let secondary = theme::button(&second_label, Btn::Secondary)
            .min_size(Vec2::new(secondary_w, 40.0));
        if ui.add_enabled(!busy && second_enabled, secondary).clicked() {
            secondary_action = second;
        }
    });

    match primary_action {
        Some(Action::Install) => app.start_install(id),
        Some(Action::Compile) => app.start_compile(id),
        Some(Action::Open) => app.open_installed(id),
        _ => {}
    }
    match secondary_action {
        Some(Action::Compile) => app.start_compile(id),
        Some(Action::InstallLatest(tag)) => {
            app.chosen.insert(id.to_string(), tag);
            app.start_install(id);
        }
        _ => {}
    }
}

enum Action {
    Install,
    Compile,
    Open,
    InstallLatest(String),
}

fn job_panel(app: &App, ui: &mut Ui, id: &str) {
    let job = app.job_for(id);
    ui.horizontal(|ui| {
        theme::section_label(ui, "Compilação");
        ui.add_space(2.0);
        if let Some(job) = job {
            ui.label(
                RichText::new(format!("· {} {}", app.branch_of(id), elapsed_label(job)))
                    .size(11.0)
                    .color(FAINT),
            );
        }
    });
    ui.add_space(6.0);

    let Some(job) = job else {
        Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(theme::r(10))
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Nenhuma compilação nesta sessão. O log aparece aqui em tempo real.")
                        .size(12.0)
                        .color(MUTED),
                );
            });
        return;
    };

    Frame::new()
        .fill(LOG_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(theme::r(10))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(format!("$ {}", job.command))
                    .monospace()
                    .size(12.0)
                    .color(ACCENT),
            );
            ui.add_space(4.0);

            let log_height = 168.0;
            Frame::new()
                .inner_margin(Margin::symmetric(0, 0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(log_height)
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .id_salt(("job-log", job.id))
                        .show(ui, |ui| {
                            if job.log.is_empty() {
                                ui.label(
                                    RichText::new("aguardando saída…")
                                        .monospace()
                                        .size(11.5)
                                        .color(FAINT),
                                );
                            }
                            for (level, line) in &job.log {
                                let color = match level {
                                    LogLevel::Info => egui::Color32::from_rgb(0xC3, 0xD3, 0xD9),
                                    LogLevel::Warn => AMBER,
                                    LogLevel::Error => DANGER,
                                };
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(line).monospace().size(11.5).color(color),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                });

            ui.add_space(8.0);
            theme::progress_bar(
                ui,
                if job.is_done() { Some(1.0) } else { job.frac },
                5.0,
                ui.input(|i| i.time),
            );
            ui.add_space(6.0);
            let status_color = match &job.done {
                Some(Ok(())) => ACCENT,
                Some(Err(_)) => DANGER,
                None => MUTED,
            };
            let text = match &job.done {
                Some(Ok(())) => format!("concluído · {}s", job.elapsed().round() as u64),
                Some(Err(err)) => format!("falhou · {err}"),
                None => match job.frac {
                    Some(frac) => format!("{}% · {}", (frac * 100.0).round() as u32, job.phase),
                    None => format!("{}s · {}", job.elapsed().round() as u64, job.phase),
                },
            };
            ui.add(
                egui::Label::new(RichText::new(text).size(11.5).color(status_color)).truncate(),
            );
        });

    if let Some(record) = &job.record {
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "instalado em {} · {}",
                record.path.display(),
                short_date(&format_epoch(record.installed_at))
            ))
            .size(11.0)
            .color(FAINT),
        );
    }
}

fn elapsed_label(job: &crate::app::JobView) -> String {
    if job.is_done() {
        "finalizada".into()
    } else {
        format!("{:.0}s", job.elapsed())
    }
}

fn format_epoch(epoch: u64) -> String {
    let days = (epoch / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{y:04}-{m:02}-{d:02}")
}

