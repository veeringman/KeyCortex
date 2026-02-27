//! Theme, skin, and form-factor management.
//!
//! Loads `themes.json`, applies CSS custom properties per skin,
//! and toggles form-factor classes on the wallet window.
//! **Must** match the JS `applySkin()` / `setThemeVars()` exactly:
//! CSS vars are set on `.wallet-window`, not on `:root`.
//! Extend by adding new skins to `themes.json` or new form factors below.

use crate::api;
use crate::dom::{self, Elements};
use crate::state;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

/// Fetch and cache `themes.json`.
pub async fn load_themes() -> Option<HashMap<String, state::ThemeTokens>> {
    let url = "../wallet-baseline/themes.json";
    let resp = api::fetch_text(url).await.ok()?;
    let map: HashMap<String, state::ThemeTokens> = serde_json::from_str(&resp).ok()?;
    state::set_themes(map.clone());
    Some(map)
}

/// Set CSS custom properties on `.wallet-window` from the given theme tokens.
/// Mirrors the JS `setThemeVars()` exactly, including property names.
pub fn set_theme_vars(els: &Elements, tokens: &state::ThemeTokens, skin: &str) {
    let root: &web_sys::HtmlElement = els.wallet_window.unchecked_ref();
    let style = root.style();

    // --wallet-skin: url('path') — the background pattern image
    if !tokens.background_pattern.is_empty() {
        let _ = style.set_property(
            "--wallet-skin",
            &format!("url('{}')", tokens.background_pattern),
        );
    }
    let _ = style.set_property("--wallet-skin-size", "12px 12px");

    // Core theme colors
    let _ = style.set_property("--edge-color", &tokens.edge);
    let _ = style.set_property("--edge-color-soft", &tokens.secondary);
    let _ = style.set_property("--glass-bg", &tokens.glass);
    let _ = style.set_property("--wallet-text", &tokens.text);
    let _ = style.set_property("--wallet-accent", &tokens.accent);

    // Optional decorative vars
    if !tokens.stitch_color.is_empty() {
        let _ = style.set_property("--stitch-color", &tokens.stitch_color);
    }
    if !tokens.clip_highlight.is_empty() {
        let _ = style.set_property("--clip-highlight", &tokens.clip_highlight);
    }
    if !tokens.clip_color.is_empty() {
        let _ = style.set_property("--clip-color", &tokens.clip_color);
    }
    if !tokens.clip_shadow.is_empty() {
        let _ = style.set_property("--clip-shadow", &tokens.clip_shadow);
    }
    if !tokens.checkered_opacity.is_empty() {
        let _ = style.set_property("--checkered-opacity", &tokens.checkered_opacity);
    }

    // Muted text color per skin (matches JS mutedMap)
    let muted = match skin {
        "dark" => "#b8a080",
        "black" => "#999",
        "navy" => "#8ba4c8",
        "forest" => "#8caa7a",
        _ => "#64748b",
    };
    let _ = style.set_property("--wallet-text-muted", muted);
}

/// Apply a named skin (e.g. "classic", "dark"). Loads themes if not cached.
pub async fn apply_skin(els: &Elements, skin: &str) {
    // Ensure themes are loaded
    let themes = match state::themes() {
        Some(t) => t,
        None => match load_themes().await {
            Some(t) => t,
            None => return,
        },
    };

    // Remove all skin classes first (matches JS: removes skin-dark/black/navy/forest)
    let skins = ["dark", "black", "navy", "forest"];
    for s in &skins {
        dom::remove_class(&els.wallet_window, &format!("skin-{}", s));
    }

    let tokens = themes.get(skin).or_else(|| themes.get("classic"));
    if let Some(tokens) = tokens {
        set_theme_vars(els, tokens, skin);
    }

    // Add skin class (only non-classic skins get a class, matching JS)
    if skin != "classic" {
        dom::add_class(&els.wallet_window, &format!("skin-{}", skin));
    }
}

/// Cycle to the next skin in the select dropdown.
pub async fn cycle_skin(els: &Elements) {
    let sel = &els.skin_select;
    let opts = sel.options();
    let len = opts.length();
    if len == 0 {
        return;
    }
    let idx = sel.selected_index();
    let next = ((idx + 1) as u32) % len;
    sel.set_selected_index(next as i32);
    let skin = dom::get_select_value(sel);
    apply_skin(els, &skin).await;
    state::local_set("kc_wallet_skin", &skin);
}

/// Apply a form factor ("pocket", "folio", "electronic").
pub fn apply_form(els: &Elements, form: &str) {
    dom::remove_class(&els.wallet_window, "form-folio");
    dom::remove_class(&els.wallet_window, "form-electronic");
    match form {
        "folio" => dom::add_class(&els.wallet_window, "form-folio"),
        "electronic" => dom::add_class(&els.wallet_window, "form-electronic"),
        _ => {} // pocket = default, no extra class
    }
}
