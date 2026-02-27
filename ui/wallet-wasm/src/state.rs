//! Global application state.
//!
//! Uses `RefCell`-wrapped `thread_local!` storage (WASM is single-threaded).
//! Extend `AppState` and the accessor helpers to add new state fields.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// ── Data models ──

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WalletInfo {
    pub wallet_address: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub bound_user_id: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IconManifest {
    #[serde(default)]
    pub networks: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub coins: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThemeTokens {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "backgroundPattern")]
    pub background_pattern: String,
    #[serde(default)]
    pub primary: String,
    #[serde(default)]
    pub secondary: String,
    #[serde(default)]
    pub edge: String,
    #[serde(default)]
    pub glass: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub accent: String,
    #[serde(default, rename = "stitchColor")]
    pub stitch_color: String,
    #[serde(default, rename = "clipHighlight")]
    pub clip_highlight: String,
    #[serde(default, rename = "clipColor")]
    pub clip_color: String,
    #[serde(default, rename = "clipShadow")]
    pub clip_shadow: String,
    #[serde(default, rename = "checkeredOpacity")]
    pub checkered_opacity: String,
}

/// Central application state.
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub wallets: Vec<WalletInfo>,
    pub profiles: Vec<Profile>,
    pub active_profile: Option<String>,
    pub active_wallet: Option<String>,
    pub manifest: Option<IconManifest>,
    pub themes: Option<std::collections::HashMap<String, ThemeTokens>>,
    pub last_challenge: Option<String>,
}

// ── Thread-local singleton ──

thread_local! {
    static STATE: RefCell<AppState> = RefCell::new(AppState::default());
}

/// Run a closure with shared read access to the state.
pub fn with<F, R>(f: F) -> R
where
    F: FnOnce(&AppState) -> R,
{
    STATE.with(|s| f(&s.borrow()))
}

/// Run a closure with mutable access to the state.
pub fn with_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut AppState) -> R,
{
    STATE.with(|s| f(&mut s.borrow_mut()))
}

// ── Convenience accessors ──

pub fn wallets() -> Vec<WalletInfo> {
    with(|s| s.wallets.clone())
}

pub fn set_wallets(w: Vec<WalletInfo>) {
    with_mut(|s| s.wallets = w);
}

pub fn active_wallet() -> Option<String> {
    with(|s| s.active_wallet.clone())
}

pub fn set_active_wallet(addr: &str) {
    with_mut(|s| s.active_wallet = Some(addr.to_string()));
}

pub fn active_profile() -> Option<String> {
    with(|s| s.active_profile.clone())
}

pub fn set_active_profile(id: &str) {
    with_mut(|s| s.active_profile = Some(id.to_string()));
}

pub fn profiles() -> Vec<Profile> {
    with(|s| s.profiles.clone())
}

pub fn set_profiles(p: Vec<Profile>) {
    with_mut(|s| s.profiles = p);
}

pub fn last_challenge() -> Option<String> {
    with(|s| s.last_challenge.clone())
}

pub fn set_last_challenge(c: Option<String>) {
    with_mut(|s| s.last_challenge = c);
}

pub fn manifest() -> Option<IconManifest> {
    with(|s| s.manifest.clone())
}

pub fn set_manifest(m: IconManifest) {
    with_mut(|s| s.manifest = Some(m));
}

pub fn themes() -> Option<std::collections::HashMap<String, ThemeTokens>> {
    with(|s| s.themes.clone())
}

pub fn set_themes(t: std::collections::HashMap<String, ThemeTokens>) {
    with_mut(|s| s.themes = Some(t));
}

// ── localStorage helpers ──

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn local_get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

pub fn local_set(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}
