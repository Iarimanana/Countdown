//! Colours, fonts and widget styling, for light and dark system themes.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Theme, Visuals,
    epaint::text::{FontTweak, VariationCoords},
};
use std::sync::Arc;

pub struct Palette {
    /// Window background behind the panels.
    pub bg: Color32,
    /// Panels and lists.
    pub surface: Color32,
    /// Text fields.
    pub field: Color32,
    /// Borders and row separators.
    pub line: Color32,
    pub ink: Color32,
    pub muted: Color32,
    /// Time remaining: the bar, the stripe, the main button.
    pub accent: Color32,
    pub on_accent: Color32,
    /// Under five minutes left.
    pub warn: Color32,
    /// Time's up.
    pub danger: Color32,
    pub on_danger: Color32,
    /// Empty part of the time bar.
    pub track: Color32,
    pub button: Color32,
    pub button_hover: Color32,
}

pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xEC, 0xEF, 0xF3),
    surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    field: Color32::from_rgb(0xF6, 0xF8, 0xFA),
    line: Color32::from_rgb(0xD6, 0xDC, 0xE4),
    ink: Color32::from_rgb(0x1B, 0x25, 0x32),
    muted: Color32::from_rgb(0x5E, 0x6B, 0x7D),
    accent: Color32::from_rgb(0x0D, 0x7A, 0x6E),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    warn: Color32::from_rgb(0xB0, 0x68, 0x00),
    danger: Color32::from_rgb(0xBF, 0x3A, 0x2B),
    on_danger: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    track: Color32::from_rgb(0xE3, 0xE8, 0xEE),
    button: Color32::from_rgb(0xF2, 0xF4, 0xF7),
    button_hover: Color32::from_rgb(0xE6, 0xEB, 0xF0),
};

pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x14, 0x1A, 0x22),
    surface: Color32::from_rgb(0x1C, 0x24, 0x2F),
    field: Color32::from_rgb(0x16, 0x1D, 0x26),
    line: Color32::from_rgb(0x2C, 0x36, 0x44),
    ink: Color32::from_rgb(0xE3, 0xE8, 0xEE),
    muted: Color32::from_rgb(0x93, 0x9F, 0xAE),
    accent: Color32::from_rgb(0x39, 0xB3, 0xA3),
    on_accent: Color32::from_rgb(0x0A, 0x1F, 0x1C),
    warn: Color32::from_rgb(0xE3, 0xA5, 0x4A),
    danger: Color32::from_rgb(0xEE, 0x6A, 0x5B),
    on_danger: Color32::from_rgb(0x24, 0x0C, 0x09),
    track: Color32::from_rgb(0x2A, 0x33, 0x40),
    button: Color32::from_rgb(0x24, 0x2D, 0x39),
    button_hover: Color32::from_rgb(0x2C, 0x37, 0x45),
};

pub fn palette(ctx: &egui::Context) -> &'static Palette {
    match ctx.theme() {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Large tabular digits for the countdown.
pub fn timer() -> TextStyle {
    TextStyle::Name("timer".into())
}

pub fn title() -> TextStyle {
    TextStyle::Name("title".into())
}

/// Section headings ("Connectés", "Nouvel appareil").
pub fn section() -> TextStyle {
    TextStyle::Name("section".into())
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, false));
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, true));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
            (TextStyle::Heading, FontId::new(17.0, bold())),
            (section(), FontId::new(17.0, bold())),
            (title(), FontId::new(23.0, bold())),
            (timer(), FontId::new(27.0, FontFamily::Name("timer".into()))),
        ]
        .into();
        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(8.0, 8.0);
        s.button_padding = egui::vec2(12.0, 6.0);
        s.interact_size.y = 30.0;
    });
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mono = |weight: f32| {
        FontData::from_static(include_bytes!("../assets/fonts/AtkinsonHyperlegibleMono.ttf")).tweak(FontTweak {
            coords: VariationCoords::new([(b"wght", weight)]),
            ..Default::default()
        })
    };
    let data = [
        ("atkinson", FontData::from_static(include_bytes!("../assets/fonts/AtkinsonHyperlegible-Regular.ttf"))),
        ("atkinson-bold", FontData::from_static(include_bytes!("../assets/fonts/AtkinsonHyperlegible-Bold.ttf"))),
        ("atkinson-mono", mono(420.0)),
        ("atkinson-mono-semibold", mono(620.0)),
    ];
    for (name, font) in data {
        fonts.font_data.insert(name.to_string(), Arc::new(font));
    }

    // egui's own fonts stay as fallbacks for any symbol Atkinson lacks.
    let defaults = fonts.families[&FontFamily::Proportional].clone();
    let with_fallback = |first: &str| {
        let mut list = vec![first.to_string()];
        list.extend(defaults.iter().cloned());
        list
    };
    fonts.families.insert(FontFamily::Proportional, with_fallback("atkinson"));
    fonts.families.insert(FontFamily::Monospace, with_fallback("atkinson-mono"));
    fonts.families.insert(bold(), with_fallback("atkinson-bold"));
    fonts.families.insert(FontFamily::Name("timer".into()), with_fallback("atkinson-mono-semibold"));
    ctx.set_fonts(fonts);
}

fn visuals(p: &Palette, dark: bool) -> Visuals {
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    let radius = CornerRadius::same(6);

    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.faint_bg_color = p.surface;
    v.extreme_bg_color = p.field;
    v.text_edit_bg_color = Some(p.field);
    v.weak_text_color = Some(p.muted);
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.danger;
    v.selection.bg_fill = p.accent.gamma_multiply(0.22);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_corner_radius = CornerRadius::same(8);
    v.window_stroke = Stroke::new(1.0, p.line);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.ink);
    w.noninteractive.corner_radius = radius;

    for (state, fill) in [(&mut w.inactive, p.button), (&mut w.hovered, p.button_hover), (&mut w.active, p.button_hover), (&mut w.open, p.button_hover)] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, p.line);
        state.fg_stroke = Stroke::new(1.0, p.ink);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    w.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.bg_stroke = Stroke::new(1.5, p.accent);
    v
}
