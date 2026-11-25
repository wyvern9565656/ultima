use egui::{
    Align2, Color32, CornerRadius, FontId, Response, RichText, Sense, Stroke,
    StrokeKind, TextStyle, Ui, Vec2,
};

// ---------------------------------------------------------------------------
// Paleta
// ---------------------------------------------------------------------------

pub const BG: Color32 = Color32::from_rgb(0x0A, 0x10, 0x16);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x0B, 0x13, 0x1B);
pub const PANEL: Color32 = Color32::from_rgb(0x0C, 0x15, 0x1D);
pub const CARD: Color32 = Color32::from_rgb(0x10, 0x1B, 0x24);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x14, 0x22, 0x2C);
pub const FIELD: Color32 = Color32::from_rgb(0x0D, 0x17, 0x20);
pub const BORDER: Color32 = Color32::from_rgb(0x1A, 0x2A, 0x35);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x2A, 0x40, 0x4D);
pub const ACCENT: Color32 = Color32::from_rgb(0x3D, 0xE0, 0xBF);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x16, 0x3B, 0x39);
pub const ACCENT_LINE: Color32 = Color32::from_rgb(0x16, 0x4A, 0x45);
pub const AMBER: Color32 = Color32::from_rgb(0xF2, 0xB7, 0x4A);
pub const DANGER: Color32 = Color32::from_rgb(0xF0, 0x7D, 0x7D);
pub const TEXT: Color32 = Color32::from_rgb(0xE6, 0xF0, 0xF2);
pub const MUTED: Color32 = Color32::from_rgb(0x8B, 0x9F, 0xAD);
pub const FAINT: Color32 = Color32::from_rgb(0x5A, 0x70, 0x7E);
pub const INK: Color32 = Color32::from_rgb(0x07, 0x10, 0x13);
pub const LOG_BG: Color32 = Color32::from_rgb(0x07, 0x0D, 0x12);

pub fn r(radius: u8) -> CornerRadius {
    CornerRadius::same(radius)
}

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(23.0, egui::FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, egui::FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, egui::FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, egui::FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, egui::FontFamily::Monospace)),
    ]
    .into();

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = FIELD;
    visuals.faint_bg_color = CARD;
    visuals.code_bg_color = LOG_BG;
    visuals.window_corner_radius = r(14);
    visuals.menu_corner_radius = r(10);
    visuals.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = AMBER;
    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = CARD_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = ACCENT_SOFT;
    visuals.widgets.active.weak_bg_fill = ACCENT_SOFT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_LINE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.open.bg_fill = CARD_HOVER;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    style.visuals = visuals;

    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.interact_size.y = 34.0;
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.floating = true;
    ctx.set_global_style(style.clone());
    ctx.all_styles_mut(|global| *global = style.clone());
}

// ---------------------------------------------------------------------------
// Ícones desenhados (sem assets externos)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Pulse,
    Nexus,
    Launcher,
    Code,
    Generic,
}

pub fn glyph_for(repo: &str) -> Glyph {
    let name = repo.rsplit('/').next().unwrap_or(repo).to_lowercase();
    if name.contains("pulse") {
        Glyph::Pulse
    } else if name.contains("launcher") {
        Glyph::Launcher
    } else if name.contains("code") {
        Glyph::Code
    } else if name.contains("nexus") {
        Glyph::Nexus
    } else {
        Glyph::Generic
    }
}

pub fn icon_tile(ui: &mut Ui, size: f32, bg: Color32, glyph: Glyph) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, r((size * 0.24) as u8), bg);

    let c = rect.center();
    let s = size;
    let stroke = Stroke::new((size * 0.075).max(1.6), INK);
    let dot = |pos: egui::Pos2, radius: f32| {
        p.circle_filled(pos, radius, INK);
    };

    match glyph {
        Glyph::Pulse => {
            let pts = vec![
                egui::pos2(c.x - s * 0.26, c.y),
                egui::pos2(c.x - s * 0.11, c.y),
                egui::pos2(c.x - s * 0.02, c.y - s * 0.17),
                egui::pos2(c.x + s * 0.08, c.y + s * 0.17),
                egui::pos2(c.x + s * 0.16, c.y),
                egui::pos2(c.x + s * 0.27, c.y),
            ];
            p.add(egui::Shape::line(pts, stroke));
        }
        Glyph::Nexus => {
            let left = egui::pos2(c.x - s * 0.17, c.y - s * 0.17);
            let upper = egui::pos2(c.x + s * 0.17, c.y - s * 0.15);
            let lower = egui::pos2(c.x + s * 0.14, c.y + s * 0.19);
            p.add(egui::Shape::line(vec![left, upper], stroke));
            p.add(egui::Shape::line(
                vec![left, egui::pos2(c.x - s * 0.02, c.y), egui::pos2(c.x - s * 0.02, c.y + s * 0.17), lower],
                stroke,
            ));
            let radius = s * 0.075;
            dot(left, radius);
            dot(upper, radius);
            dot(lower, radius);
        }
        Glyph::Launcher => {
            let body = egui::Rect::from_center_size(c, egui::vec2(s * 0.56, s * 0.34));
            p.rect_filled(body, r((s * 0.1) as u8), INK);
            p.rect_filled(body.shrink(s * 0.05), r((s * 0.07) as u8), bg);
            let arm_x = c.x - s * 0.12;
            let arm_y = c.y - s * 0.02;
            p.line_segment(
                [egui::pos2(arm_x - s * 0.06, arm_y), egui::pos2(arm_x + s * 0.06, arm_y)],
                Stroke::new(s * 0.045, INK),
            );
            p.line_segment(
                [egui::pos2(arm_x, arm_y - s * 0.06), egui::pos2(arm_x, arm_y + s * 0.06)],
                Stroke::new(s * 0.045, INK),
            );
            dot(egui::pos2(c.x + s * 0.12, c.y - s * 0.04), s * 0.032);
            dot(egui::pos2(c.x + s * 0.17, c.y + s * 0.05), s * 0.032);
        }
        Glyph::Code => {
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x + s * 0.04, c.y - s * 0.24),
                    egui::pos2(c.x - s * 0.1, c.y + s * 0.24),
                ],
                stroke,
            ));
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - s * 0.14, c.y - s * 0.13),
                    egui::pos2(c.x - s * 0.26, c.y),
                    egui::pos2(c.x - s * 0.14, c.y + s * 0.13),
                ],
                stroke,
            ));
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x + s * 0.14, c.y - s * 0.13),
                    egui::pos2(c.x + s * 0.26, c.y),
                    egui::pos2(c.x + s * 0.14, c.y + s * 0.13),
                ],
                stroke,
            ));
        }
        Glyph::Generic => {
            let box_rect = egui::Rect::from_center_size(c, egui::vec2(s * 0.42, s * 0.42));
            p.rect_stroke(box_rect, r(4), stroke, StrokeKind::Inside);
            p.line_segment(
                [
                    egui::pos2(box_rect.center().x - box_rect.width() * 0.5, c.y),
                    egui::pos2(box_rect.center().x + box_rect.width() * 0.5, c.y),
                ],
                stroke,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Botões
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    /// Ação principal: preenchido com o accent.
    Primary,
    /// Contorno sobre o card.
    Secondary,
    /// Atualização disponível.
    Amber,
    /// Ação discreta sobre fundo escuro.
    Ghost,
}

pub fn button(text: impl Into<String>, kind: Btn) -> egui::Button<'static> {
    let text = text.into();
    match kind {
        Btn::Primary => egui::Button::new(RichText::new(text).color(INK).strong())
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(r(8)),
        Btn::Amber => egui::Button::new(RichText::new(text).color(INK).strong())
            .fill(AMBER)
            .stroke(Stroke::NONE)
            .corner_radius(r(8)),
        Btn::Secondary => egui::Button::new(RichText::new(text).color(TEXT))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, BORDER_STRONG))
            .corner_radius(r(8)),
        Btn::Ghost => egui::Button::new(RichText::new(text).color(MUTED))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(r(8)),
    }
}

pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let (fg, fill, stroke) = if selected {
        (ACCENT, ACCENT_SOFT, ACCENT_LINE)
    } else {
        (MUTED, Color32::TRANSPARENT, BORDER)
    };
    ui.add(
        egui::Button::new(RichText::new(label).color(fg).size(13.5))
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(r(16))
            .min_size(Vec2::new(0.0, 32.0))
            .selected(selected),
    )
}

// ---------------------------------------------------------------------------
// Estruturas de apoio
// ---------------------------------------------------------------------------

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(11.0).color(FAINT).strong());
}

pub fn nav_row(ui: &mut Ui, label: &str, selected: bool, badge: Option<usize>) -> Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let fill = if selected {
        ACCENT_SOFT
    } else if resp.hovered() {
        CARD
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, r(8), fill);
    }
    if selected {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 9.0),
            egui::vec2(3.0, rect.height() - 18.0),
        );
        ui.painter().rect_filled(bar, r(2), ACCENT);
    }
    let color = if selected { ACCENT } else { TEXT };
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(15.0),
        color,
    );

    if let Some(count) = badge {
        if count > 0 {
            let badge_rect = egui::Rect::from_center_size(
                rect.right_center() - egui::vec2(19.0, 0.0),
                egui::vec2(24.0, 20.0),
            );
            ui.painter().rect_filled(badge_rect, r(10), AMBER);
            ui.painter().text(
                badge_rect.center(),
                Align2::CENTER_CENTER,
                count.to_string(),
                FontId::proportional(12.0),
                INK,
            );
        }
    }
    resp
}

pub fn progress_bar(ui: &mut Ui, fraction: Option<f32>, height: f32, ctx_time: f64) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, r((height / 2.0) as u8), Color32::from_rgb(0x1B, 0x28, 0x31));
    let clamped = fraction.unwrap_or_else(|| {
        // Barra indeterminada: um trecho que percorre a trilha.
        let cycle = (ctx_time * 0.45) % 1.0;
        let head = 0.15 + 0.7 * cycle;
        return head.clamp(0.08, 0.95) as f32;
    });
    let clamped = clamped.clamp(0.0, 1.0);
    let mut fill = rect;
    fill.set_width((rect.width() * clamped).max(height));
    p.rect_filled(fill, r((height / 2.0) as u8), ACCENT);
}

pub fn dot(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter().circle_filled(rect.center(), size / 2.0, color);
}
