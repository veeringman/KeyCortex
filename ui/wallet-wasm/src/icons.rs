//! Icon resolver and manifest loading.
//!
//! Loads `config/icon-manifest.json` and resolves network/coin icon paths.

use crate::api;
use crate::dom::{self, Elements};
use crate::state;

/// Load the icon manifest from `../../config/icon-manifest.json`.
pub async fn load_manifest() {
    let url = "../../config/icon-manifest.json";
    match api::fetch_text(url).await {
        Ok(text) => {
            if let Ok(m) = serde_json::from_str::<state::IconManifest>(&text) {
                state::set_manifest(m);
            }
        }
        Err(_) => {} // Non-critical, silently ignore
    }
}

/// Resolve network icon path.
pub fn resolve_network_icon(chain: &str) -> String {
    if let Some(manifest) = state::manifest() {
        if let Some(path) = manifest.networks.get(chain) {
            return path.clone();
        }
    }
    // Fallback
    format!("../../assets/icons/networks/{}.svg", chain)
}

/// Normalise asset name (e.g. FloweR → flower).
pub fn normalize_asset(asset: &str) -> String {
    match asset {
        "FloweR" => "flower".to_string(),
        other => other.to_lowercase(),
    }
}

/// Resolve coin icon path.
pub fn resolve_coin_icon(asset: &str) -> String {
    let key = normalize_asset(asset);
    if let Some(manifest) = state::manifest() {
        if let Some(path) = manifest.coins.get(&key) {
            return path.clone();
        }
    }
    format!("../../assets/icons/coins/{}.svg", key)
}

/// Update balance icon `<img>` sources based on current field values.
pub fn update_balance_icons(els: &Elements) {
    let chain = dom::get_input_value(&els.balance_chain);
    let asset = dom::get_select_value(&els.balance_asset);
    let chain_key = if chain.is_empty() {
        "flowcortex-l1".to_string()
    } else {
        chain
    };
    els.balance_network_icon
        .set_src(&resolve_network_icon(&chain_key));
    els.balance_coin_icon
        .set_src(&resolve_coin_icon(&asset));
}
