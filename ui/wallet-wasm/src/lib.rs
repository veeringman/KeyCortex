//! KeyCortex Wallet WASM Frontend
//!
//! Pure Rust + WASM implementation replicating the JavaScript wallet-baseline UI.
//! Modularised for extensibility: each concern lives in its own module.

pub mod api;
pub mod dom;
pub mod events;
pub mod fold;
pub mod icons;
pub mod platform;
pub mod profile;
pub mod state;
pub mod theme;
pub mod wallet_list;
pub mod wallet_ops;

use wasm_bindgen::prelude::*;

/// WASM entry point – called automatically when the module is instantiated.
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    // Improve panic messages in the browser console
    console_error_panic_hook::set_once();

    init().await
}

/// Main initialisation sequence (mirrors JS `main()`).
async fn init() -> Result<(), JsValue> {
    let els = dom::Elements::bind()?;

    // Set initial fold state to folded (must be first, before anything else renders)
    fold::set_wallet_state(&els, fold::FoldState::Folded);

    // Restore skin
    let saved_skin = state::local_get("kc_wallet_skin").unwrap_or_default();
    if !saved_skin.is_empty() {
        if dom::has_option(&els.skin_select, &saved_skin) {
            dom::set_select_value(&els.skin_select, &saved_skin);
        }
    }
    let skin = dom::get_select_value(&els.skin_select);
    theme::apply_skin(&els, &skin).await;

    // Restore form factor
    let saved_form = state::local_get("kc_wallet_form").unwrap_or_default();
    if !saved_form.is_empty() {
        if dom::has_option(&els.form_select, &saved_form) {
            dom::set_select_value(&els.form_select, &saved_form);
        }
    }
    let form = dom::get_select_value(&els.form_select);
    theme::apply_form(&els, &form);

    // Load profiles and wallets
    profile::load_profiles(&els);
    wallet_list::load_wallet_list(&els).await;

    // Restore last active wallet
    let saved_wallet = state::local_get("kc_active_wallet").unwrap_or_default();
    let wallets = state::wallets();
    if !saved_wallet.is_empty() && wallets.iter().any(|w| w.wallet_address == saved_wallet) {
        wallet_list::select_active_wallet(&els, &saved_wallet);
    } else if !wallets.is_empty() {
        wallet_list::select_active_wallet(&els, &wallets[0].wallet_address.clone());
    }

    // Bind all event listeners
    events::bind_events(&els);

    // Load icon manifest
    icons::load_manifest().await;

    Ok(())
}
