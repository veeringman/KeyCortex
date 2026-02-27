//! Profile management.
//!
//! Profiles are stored in `localStorage`. Each profile can have wallets assigned.
//! Extend by adding multi-device sync or backend-backed profiles.

use crate::dom::{self, Elements};
use crate::state;
use crate::wallet_list;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

// ── Profile CRUD ──

/// Load profiles from localStorage, ensuring at least one default profile exists.
pub fn load_profiles(els: &Elements) {
    let raw = state::local_get("kc_profiles").unwrap_or_else(|| "[]".to_string());
    let mut profiles: Vec<state::Profile> = serde_json::from_str(&raw).unwrap_or_default();

    if profiles.is_empty() {
        profiles.push(state::Profile {
            id: "default".into(),
            name: "Default User".into(),
        });
        save_profiles(&profiles);
    }

    state::set_profiles(profiles.clone());

    let active = state::local_get("kc_active_profile")
        .unwrap_or_else(|| profiles[0].id.clone());
    state::set_active_profile(&active);

    render_profile_select(els);
}

pub fn save_profiles(profiles: &[state::Profile]) {
    let json = serde_json::to_string(profiles).unwrap_or_else(|_| "[]".into());
    state::local_set("kc_profiles", &json);
}

pub fn render_profile_select(els: &Elements) {
    let sel = &els.profile_select;
    dom::set_inner_html(sel.unchecked_ref(), "");

    let profiles = state::profiles();
    let active = state::active_profile().unwrap_or_default();

    for p in &profiles {
        let opt = dom::create_option(&p.id, &p.name, p.id == active);
        sel.append_child(&opt).unwrap();
    }
}

/// Handle profile change from the dropdown.
pub async fn on_profile_change(els: &Elements) {
    let id = dom::get_select_value(&els.profile_select);
    state::set_active_profile(&id);
    state::local_set("kc_active_profile", &id);
    wallet_list::load_wallet_list(els).await;
    wallet_list::update_half_fold_info(els);
}

/// Add a new profile (prompts user for name).
pub fn on_add_profile(els: &Elements) {
    let name = dom::window()
        .prompt_with_message("Enter profile / user name:")
        .ok()
        .flatten()
        .unwrap_or_default();
    if name.trim().is_empty() {
        return;
    }

    let id = format!("profile-{}", js_sys::Date::now() as u64);
    let mut profiles = state::profiles();
    profiles.push(state::Profile {
        id: id.clone(),
        name: name.trim().to_string(),
    });
    save_profiles(&profiles);
    state::set_profiles(profiles);
    state::set_active_profile(&id);
    state::local_set("kc_active_profile", &id);

    render_profile_select(els);

    {
        let els2 = els.clone();
        wasm_bindgen_futures::spawn_local(async move {
            wallet_list::load_wallet_list(&els2).await;
            wallet_list::update_half_fold_info(&els2);
        });
    }
}

// ── Profile–Wallet mapping ──

fn get_profile_wallet_map() -> HashMap<String, Vec<String>> {
    let raw = state::local_get("kc_profile_wallets").unwrap_or_else(|| "{}".into());
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_profile_wallet_map(map: &HashMap<String, Vec<String>>) {
    let json = serde_json::to_string(map).unwrap_or_else(|_| "{}".into());
    state::local_set("kc_profile_wallets", &json);
}

pub fn assign_wallet_to_profile(wallet_address: &str, profile_id: &str) {
    let mut map = get_profile_wallet_map();
    let list = map.entry(profile_id.to_string()).or_default();
    if !list.contains(&wallet_address.to_string()) {
        list.push(wallet_address.to_string());
    }
    save_profile_wallet_map(&map);
}

pub fn unassign_wallet_from_profile(wallet_address: &str, profile_id: &str) {
    let mut map = get_profile_wallet_map();
    if let Some(list) = map.get_mut(profile_id) {
        list.retain(|a| a != wallet_address);
    }
    save_profile_wallet_map(&map);
}

/// Get assigned and unassigned wallets for a profile.
pub fn get_wallets_for_profile(
    profile_id: &str,
) -> (Vec<state::WalletInfo>, Vec<state::WalletInfo>) {
    let map = get_profile_wallet_map();
    let assigned_addrs = map.get(profile_id).cloned().unwrap_or_default();
    let all_assigned: Vec<String> = map.values().flatten().cloned().collect();

    let wallets = state::wallets();
    let assigned: Vec<state::WalletInfo> = wallets
        .iter()
        .filter(|w| assigned_addrs.contains(&w.wallet_address))
        .cloned()
        .collect();
    let unassigned: Vec<state::WalletInfo> = wallets
        .iter()
        .filter(|w| !all_assigned.contains(&w.wallet_address))
        .cloned()
        .collect();

    (assigned, unassigned)
}

/// Get profile name by id.
pub fn get_profile_name(profile_id: &str) -> String {
    state::profiles()
        .iter()
        .find(|p| p.id == profile_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| profile_id.to_string())
}
