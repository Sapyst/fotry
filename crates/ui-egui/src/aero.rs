//! Fotry (Arch Quilly): the Aero theme, our own look on the Studio layout.
//!
//! Flat neutral greys taken from the Arch Aero desktop (the same #1e1e1e / #f6f6f6 the panels and
//! the GTK theme use), rounder corners, and the desktop's accent colour. The accent is read once at
//! start-up from `~/.cache/hypr-tema/cores` (written by the desktop's `hypr-tema`); "mono" or a
//! missing file means a monochrome accent: near-white on dark, near-black on light.
//!
//! PhotoCraft draws Aero with its own palette ([`tokens`], through `theme::Palette`), so the
//! accent follows the desktop. The suite only lists themes it knows (built-ins and the user's
//! theme library), so [`install_library_theme`] also keeps `library/aero.json` in the suite's
//! theme folder: that is what puts Aero in the theme menus and lets the suite selection hold it.

use egui::Color32;
use std::sync::OnceLock;

use crate::theme::Tokens;

/// The suite theme id of Aero (dark and light variants).
pub const ID: &str = "aero";

/// The desktop's accent as RGB, or `None` for the monochrome accent.
fn desktop_accent() -> Option<[u8; 3]> {
    static ACCENT: OnceLock<Option<[u8; 3]>> = OnceLock::new();
    *ACCENT.get_or_init(read_accent)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_accent() -> Option<[u8; 3]> {
    let home = std::env::var_os("HOME")?;
    let text = std::fs::read_to_string(std::path::Path::new(&home).join(".cache/hypr-tema/cores")).ok()?;
    // "COR_SINC COR_CLARA COR_ESCURA COR_ALFA COR_DESTAQUE BAR_ALPHA"
    parse_hex(text.split_whitespace().nth(4)?)
}

#[cfg(target_arch = "wasm32")]
fn read_accent() -> Option<[u8; 3]> {
    None
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    let byte = |i: usize| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Text that reads on top of `c`: dark on light colours, white on dark ones (the desktop's rule).
fn on(c: [u8; 3]) -> Color32 {
    let lum = 299 * u32::from(c[0]) + 587 * u32::from(c[1]) + 114 * u32::from(c[2]);
    if lum > 165_000 { Color32::from_rgb(30, 30, 30) } else { Color32::WHITE }
}

/// The Aero tokens in the dark or light mode.
pub fn tokens(dark: bool) -> Tokens {
    let g = Color32::from_gray;
    let mono = if dark { [240, 240, 240] } else { [30, 30, 30] };
    let accent = desktop_accent().unwrap_or(mono);
    let [r, gr, b] = accent;
    let a = |alpha: u8| Color32::from_rgba_unmultiplied(r, gr, b, alpha);
    if dark {
        Tokens {
            dark,
            chrome: g(30),
            canvas: g(20),
            dock: g(25),
            card: g(36),
            card_border: g(50),
            field: g(44),
            field_border: g(62),
            hover: g(50),
            pressed: g(60),
            text: g(240),
            text_dim: g(208),
            text_faint: g(176),
            icon: g(214),
            accent: Color32::from_rgb(r, gr, b),
            accent_soft: a(40),
            accent_border: a(150),
            accent_text: if accent == mono { g(240) } else { Color32::from_rgb(r, gr, b) },
            separator: g(22),
            shadow: Color32::from_black_alpha(120),
            primary_bg: Color32::from_rgb(r, gr, b),
            primary_text: on(accent),
            danger: Color32::from_rgb(255, 128, 128),
            warning: Color32::from_rgb(240, 196, 120),
            radius_sm: 8.0,
            radius: 12.0,
            radius_lg: 16.0,
            bevel: false,
            pro: false,
            tab_strip: Color32::TRANSPARENT,
            row_selected: Color32::TRANSPARENT,
            histogram_bg: g(20),
            histogram_level: 220,
            caption_close: Color32::from_rgb(200, 64, 64),
            caption_close_text: Color32::WHITE,
            scrim: Color32::from_black_alpha(110),
        }
    } else {
        Tokens {
            dark,
            chrome: g(246),
            canvas: g(226),
            dock: g(238),
            card: g(250),
            card_border: g(220),
            field: g(255),
            field_border: g(196),
            hover: g(236),
            pressed: g(222),
            text: g(30),
            text_dim: g(58),
            text_faint: g(88),
            icon: g(52),
            accent: Color32::from_rgb(r, gr, b),
            accent_soft: a(34),
            accent_border: a(170),
            accent_text: if accent == mono { g(30) } else { Color32::from_rgb(r, gr, b) },
            separator: g(214),
            shadow: Color32::from_black_alpha(40),
            primary_bg: Color32::from_rgb(r, gr, b),
            primary_text: on(accent),
            danger: Color32::from_rgb(170, 40, 40),
            warning: Color32::from_rgb(140, 72, 0),
            radius_sm: 8.0,
            radius: 12.0,
            radius_lg: 16.0,
            bevel: false,
            pro: false,
            tab_strip: Color32::TRANSPARENT,
            row_selected: Color32::TRANSPARENT,
            histogram_bg: g(30),
            histogram_level: 240,
            caption_close: Color32::from_rgb(200, 56, 56),
            caption_close_text: Color32::WHITE,
            scrim: Color32::from_black_alpha(90),
        }
    }
}

fn hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") }
}

/// One variant of the suite theme file, from the same tokens PhotoCraft draws with.
fn variant(t: &Tokens) -> serde_json::Value {
    serde_json::json!({
        "accent.border": hex(t.accent_border),
        "accent.soft": hex(t.accent_soft),
        "accent.solid": hex(t.accent),
        "accent.text": hex(t.accent_text),
        "caption.close_hover": hex(t.caption_close),
        "caption.close_text": hex(t.caption_close_text),
        "line.border": hex(t.card_border),
        "line.control_edge": hex(t.field_border),
        "line.divider": hex(t.separator),
        "state.hover": hex(t.hover),
        "state.pressed": hex(t.pressed),
        "state.selected": "{accent.soft}",
        "state.selected_hover": "{accent.soft}",
        "surface.background": hex(t.dock),
        "surface.canvas": hex(t.canvas),
        "surface.canvas_pattern": hex(t.canvas),
        "surface.chrome": hex(t.chrome),
        "surface.panel": hex(t.card),
        "surface.sunken": hex(t.field),
        "text.faint": hex(t.text_faint),
        "text.muted": hex(t.text_dim),
        "text.normal": hex(t.text),
        "text.on_strong": hex(t.primary_text),
    })
}

/// The Aero suite theme file (`craft-design-theme`), with the desktop's current accent.
pub fn library_json() -> String {
    let (dark, light) = (tokens(true), tokens(false));
    let seed = |c: Color32| hex(Color32::from_rgb(c.r(), c.g(), c.b()));
    let doc = serde_json::json!({
        "schema": "craft-design-theme",
        "schema_version": 1,
        "id": ID,
        "name": "Aero",
        "version": 1,
        "variants": "both",
        "contrast_intent": "standard",
        "seeds": {
            "accent": seed(dark.accent),
            "neutral": seed(dark.chrome),
            "status": { "bad": seed(dark.danger), "good": "#85dc6a", "info": seed(dark.accent), "warn": seed(dark.warning) }
        },
        "shape": { "button_radius": dark.radius_sm },
        "light": variant(&light),
        "dark": variant(&dark),
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

/// Writes `library/aero.json` into the suite theme folder `dir` when it is missing or its accent
/// is no longer the desktop's. Errors are logged: the palette still draws Aero without it.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_library_theme(dir: Option<&std::path::Path>) {
    let Some(dir) = dir else { return };
    let path = dir.join("library").join(format!("{ID}.json"));
    let json = library_json();
    if std::fs::read_to_string(&path).is_ok_and(|old| old == json) {
        return;
    }
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(path.parent().unwrap_or(dir))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)
    };
    if let Err(e) = write() {
        log::warn!("Aero theme: could not write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_parses_and_mono_does_not() {
        assert_eq!(super::parse_hex("3584e4"), Some([0x35, 0x84, 0xe4]));
        assert_eq!(super::parse_hex("mono"), None);
        assert_eq!(super::parse_hex("zzzzzz"), None);
    }

    /// The theme file reads as a suite theme, so the menus can list Aero.
    #[test]
    fn library_theme_parses() {
        let def = craft::theme::format::parse(super::library_json().as_bytes());
        assert!(def.is_ok(), "{def:?}");
    }
}
