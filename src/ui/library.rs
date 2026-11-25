use egui::{Align, Label, Layout, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::app::{App, Page};
use crate::model::display_tag;
use crate::theme::{self, Btn, ACCENT, AMBER, BORDER, BORDER_STRONG, CARD, CARD_HOVER, DANGER, MUTED, TEXT};

#[derive(Clone)]
enum CardAction {
    Install,
    Update,
    Open,
    Compile,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    let ids = app.filtered_repos();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("library-scroll")
        .show(ui, |ui| {
            if ids.is_empty() {
                empty_state(app, ui);
                return;
            }
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
            let avail = ui.available_width();
            let cols = (((avail + 14.0) / (330.0 + 14.0)).floor() as usize).clamp(1, 4);
            let card_w = (avail - 14.0 * (cols.saturating_sub(1)) as f32) / cols as f32;

            for chunk in ids.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    for id in chunk {
                        card(app, ui, id, card_w);
                    }
                });
            }
            if app.page == Page::Library {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    add_card(app, ui, card_w);
                });
            }
        });
}

fn card(app: &mut App, ui: &mut Ui, id: &str, width: f32) {
    let Some(repo) = app.repo(id).cloned() else {
        return;
    };
    let installed = app.installs.get(id).cloned();
    let latest = app.latest_stable(id).cloned();
    let has_update = app.has_updates(id);
    let busy = app.active_job_for(id).map(|j| j.phase.clone());
    let error = app.errors.get(id).cloned();
    let selected = app.selected.as_deref() == Some(id);
    let subtitle = app.subtitle_for(id);
    let description = app
        .infos
        .get(id)
        .and_then(|i| i.description.clone())
        .filter(|d| !d.is_empty() && *d != subtitle)
        .unwrap_or_else(|| {
            if subtitle.is_empty() {
                "Repositório conectado".into()
            } else {
                subtitle.clone()
            }
        });
    let accent = crate::model::accent_for(id);

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 212.0), Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    painter.rect_filled(rect, theme::r(12), if hovered { CARD_HOVER } else { CARD });
    let border = if selected {
        ACCENT
    } else if hovered {
        BORDER_STRONG
    } else {
        BORDER
    };
    painter.rect_stroke(
        rect,
        theme::r(12),
        Stroke::new(if selected { 1.4 } else { 1.0 }, border),
        StrokeKind::Inside,
    );

    let inner = rect.shrink2(Vec2::new(18.0, 16.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    child.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

    child.horizontal(|ui| {
        theme::icon_tile(ui, 46.0, accent, theme::glyph_for(&repo.full_name));
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.add_space(1.0);
            ui.label(RichText::new(repo.title()).size(17.0).strong().color(TEXT));
            ui.label(RichText::new(&subtitle).size(12.0).color(MUTED));
        });
    });

    child.add_space(10.0);
    let wrapped = wrap_two_lines(&description, (width / 6.6) as usize);
    child.add(Label::new(RichText::new(wrapped).size(12.8).color(MUTED)).wrap());

    let (status_text, status_color, show_dot, action) = if let Some(phase) = &busy {
        (
            format!("{}…", short_phase(phase)),
            ACCENT,
            true,
            None,
        )
    } else if error.is_some() {
        ("Falha ao consultar".to_string(), DANGER, true, None)
    } else if has_update {
        (
            format!(
                "Atualização · {} → {}",
                installed.as_ref().map(|r| r.label()).unwrap_or_default(),
                latest.as_ref().map(|r| display_tag(&r.tag)).unwrap_or_default()
            ),
            AMBER,
            true,
            Some(CardAction::Update),
        )
    } else if let Some(record) = &installed {
        (
            format!("Instalado · {}", record.label()),
            ACCENT,
            true,
            Some(CardAction::Open),
        )
    } else if let Some(release) = &latest {
        (
            format!("Não instalado · {}", display_tag(&release.tag)),
            MUTED,
            false,
            Some(CardAction::Install),
        )
    } else {
        (
            format!("Só código · {}", app.branch_of(id)),
            MUTED,
            false,
            Some(CardAction::Compile),
        )
    };

    let button_plan = if busy.is_some() {
        None
    } else {
        match &action {
            Some(CardAction::Update) => Some(("Atualizar", Btn::Amber)),
            Some(CardAction::Open) => Some(("Abrir", Btn::Secondary)),
            Some(CardAction::Install) => Some(("Instalar", Btn::Primary)),
            Some(CardAction::Compile) => Some(("Compilar", Btn::Secondary)),
            None => None,
        }
    };

    let footer = 34.0;
    let remaining = child.available_height() - footer;
    if remaining > 0.0 {
        child.add_space(remaining);
    }

    let mut clicked_action: Option<CardAction> = None;
    child.horizontal(|ui| {
        ui.horizontal(|ui| {
            if show_dot {
                theme::dot(ui, status_color, 7.0);
            }
            ui.add(Label::new(RichText::new(&status_text).size(12.0).color(status_color)).truncate());
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some((label, kind)) = button_plan {
                if ui
                    .add(theme::button(label, kind).min_size(Vec2::new(84.0, 32.0)))
                    .clicked()
                {
                    clicked_action = action.clone();
                }
            } else {
                ui.add(egui::Spinner::new().size(14.0));
            }
        });
    });

    if response.clicked() {
        app.select_repo(id);
    }
    if let Some(err) = &error {
        response.clone().on_hover_text(err);
    }

    match clicked_action {
        Some(CardAction::Install) => {
            if let Some(release) = &latest {
                app.chosen.insert(id.to_string(), release.tag.clone());
            }
            app.select_repo(id);
            app.start_install(id);
        }
        Some(CardAction::Update) => {
            app.select_repo(id);
            app.start_update(id);
        }
        Some(CardAction::Open) => {
            app.select_repo(id);
            app.open_installed(id);
        }
        Some(CardAction::Compile) => {
            app.select_repo(id);
            app.start_compile(id);
        }
        None => {}
    }
}

fn add_card(app: &mut App, ui: &mut Ui, width: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 212.0), Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter()
        .rect_filled(rect, theme::r(12), if hovered { CARD_HOVER } else { CARD });
    dashed_border(
        ui.painter(),
        rect,
        Stroke::new(1.0, if hovered { ACCENT } else { BORDER_STRONG }),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "+ Adicionar repositório",
        egui::FontId::proportional(14.0),
        if hovered { ACCENT } else { MUTED },
    );
    if response.clicked() {
        app.token_draft = app.config.token.clone().unwrap_or_default();
        app.dir_draft = app.config.install_dir.display().to_string();
        app.settings_error = None;
        app.show_settings = true;
    }
}

fn dashed_border(painter: &egui::Painter, rect: egui::Rect, stroke: Stroke) {
    let r = 12.0;
    let dash = 7.0;
    let gap = 5.0;
    let inset = rect.shrink(0.5);
    let mut segments: Vec<[egui::Pos2; 2]> = Vec::new();

    let edges = [
        (
            egui::pos2(inset.left() + r, inset.top()),
            egui::pos2(inset.right() - r, inset.top()),
        ),
        (
            egui::pos2(inset.right(), inset.top() + r),
            egui::pos2(inset.right(), inset.bottom() - r),
        ),
        (
            egui::pos2(inset.right() - r, inset.bottom()),
            egui::pos2(inset.left() + r, inset.bottom()),
        ),
        (
            egui::pos2(inset.left(), inset.bottom() - r),
            egui::pos2(inset.left(), inset.top() + r),
        ),
    ];
    for (from, to) in edges {
        let length = (to - from).length();
        let dir = (to - from) / length.max(1.0);
        let mut travelled = 0.0;
        while travelled < length {
            let end = (travelled + dash).min(length);
            segments.push([from + dir * travelled, from + dir * end]);
            travelled += dash + gap;
        }
    }
    for segment in segments {
        painter.line_segment(segment, stroke);
    }

    // Cantos arredondados discretos.
    for (center, start_angle) in [
        (egui::pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI),
        (egui::pos2(rect.right() - r, rect.top() + r), 1.5 * std::f32::consts::PI),
        (egui::pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (egui::pos2(rect.left() + r, rect.bottom() - r), 0.5 * std::f32::consts::PI),
    ] {
        let points: Vec<egui::Pos2> = (0..=4)
            .map(|i| {
                let angle = start_angle + (std::f32::consts::FRAC_PI_2) * (i as f32 / 4.0);
                center + egui::vec2(angle.cos(), angle.sin()) * r
            })
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
}

fn empty_state(app: &App, ui: &mut Ui) {
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        let message = match app.page {
            Page::Updates => "Tudo atualizado — nenhum app com versão nova.",
            _ => "Nenhum repositório corresponde a este filtro.",
        };
        ui.label(RichText::new(message).size(14.0).color(MUTED));
        ui.add_space(6.0);
        ui.label(
            RichText::new("Conecte outros repositórios em Configurações.")
                .size(12.5)
                .color(theme::FAINT),
        );
    });
}

fn short_phase(phase: &str) -> String {
    let char_count = phase.chars().count();
    if char_count > 34 {
        format!("{}…", phase.chars().take(33).collect::<String>())
    } else {
        phase.to_string()
    }
}

fn wrap_two_lines(text: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(18);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut truncated = false;
    for word in text.split_whitespace() {
        let projected = current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if projected > max_chars && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            if lines.len() == 2 {
                truncated = true;
                break;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if lines.len() < 2 && !current.is_empty() {
        lines.push(current);
    }
    if truncated {
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    lines.join("\n")
}

