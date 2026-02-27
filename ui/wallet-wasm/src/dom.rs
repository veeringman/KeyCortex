//! DOM element bindings.
//!
//! Mirrors the JS `elements` object. All fields are resolved once at startup.
//! To add new UI elements, add a field here and bind it in `Elements::bind()`.

use wasm_bindgen::prelude::*;
use web_sys::{
    Document, Element, HtmlElement, HtmlImageElement, HtmlInputElement,
    HtmlOptionElement, HtmlSelectElement, HtmlTextAreaElement,
};

// ── Helpers ──

fn doc() -> Document {
    web_sys::window().unwrap().document().unwrap()
}

pub fn by_id(id: &str) -> Option<Element> {
    doc().get_element_by_id(id)
}

pub fn by_id_typed<T: JsCast>(id: &str) -> Option<T> {
    by_id(id).and_then(|e| e.dyn_into::<T>().ok())
}

pub fn query(selector: &str) -> Option<Element> {
    doc().query_selector(selector).ok()?
}

pub fn query_all(selector: &str) -> Vec<Element> {
    let nl = doc().query_selector_all(selector).unwrap();
    let mut v = Vec::new();
    for i in 0..nl.length() {
        if let Some(e) = nl.item(i) {
            if let Ok(el) = e.dyn_into::<Element>() {
                v.push(el);
            }
        }
    }
    v
}

pub fn set_text(el: &Element, text: &str) {
    el.set_text_content(Some(text));
}

pub fn set_inner_html(el: &Element, html: &str) {
    el.set_inner_html(html);
}

pub fn set_input_value(el: &HtmlInputElement, val: &str) {
    el.set_value(val);
}

pub fn get_input_value(el: &HtmlInputElement) -> String {
    el.value().trim().to_string()
}

pub fn get_select_value(el: &HtmlSelectElement) -> String {
    el.value()
}

pub fn set_select_value(el: &HtmlSelectElement, val: &str) {
    el.set_value(val);
}

pub fn get_textarea_value(el: &HtmlTextAreaElement) -> String {
    el.value().trim().to_string()
}

pub fn has_option(sel: &HtmlSelectElement, value: &str) -> bool {
    let opts = sel.options();
    for i in 0..opts.length() {
        if let Some(opt) = opts.item(i) {
            if let Ok(o) = opt.dyn_into::<HtmlOptionElement>() {
                if o.value() == value {
                    return true;
                }
            }
        }
    }
    false
}

pub fn add_class(el: &Element, cls: &str) {
    let _ = el.class_list().add_1(cls);
}

pub fn remove_class(el: &Element, cls: &str) {
    let _ = el.class_list().remove_1(cls);
}

pub fn toggle_class(el: &Element, cls: &str, force: bool) {
    let _ = el.class_list().toggle_with_force(cls, force);
}

pub fn has_class(el: &Element, cls: &str) -> bool {
    el.class_list().contains(cls)
}

pub fn create_element(tag: &str) -> Element {
    doc().create_element(tag).unwrap()
}

/// Query all matching elements within a parent element.
pub fn query_all_within(parent: &Element, selector: &str) -> Vec<Element> {
    let nl = parent.query_selector_all(selector).unwrap();
    let mut v = Vec::new();
    for i in 0..nl.length() {
        if let Some(e) = nl.item(i) {
            if let Ok(el) = e.dyn_into::<Element>() {
                v.push(el);
            }
        }
    }
    v
}

pub fn create_option(value: &str, text: &str, selected: bool) -> HtmlOptionElement {
    let opt: HtmlOptionElement = create_element("option").dyn_into().unwrap();
    opt.set_value(value);
    opt.set_text_content(Some(text));
    opt.set_selected(selected);
    opt
}

pub fn document() -> Document {
    doc()
}

pub fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}

// ── Elements struct ──

/// All DOM element references used by the wallet UI.
/// Clone-friendly (all inner types are reference-counted via JS GC).
#[derive(Clone)]
pub struct Elements {
    // Layout
    pub wallet_window: Element,
    pub wallet_folded: Element,
    pub wallet_fold_toggle: Element,
    pub wallet_app: Element,

    // Header / config
    pub base_url: HtmlInputElement,
    pub skin_select: HtmlSelectElement,
    pub form_select: HtmlSelectElement,
    pub skin_cycle_btn: HtmlElement,

    // Tabs
    pub tabs: Vec<Element>,
    pub panels: Vec<Element>,

    // Wallet create
    pub create_wallet_btn: HtmlElement,
    pub create_result: Element,
    pub wallet_label_input: HtmlInputElement,
    pub wallet_passphrase_input: HtmlInputElement,
    pub restore_wallet_btn: HtmlElement,
    pub restore_hint: Element,
    pub refresh_wallets_btn: HtmlElement,

    // Wallet list
    pub wallet_list_container: Element,

    // Half-fold info
    pub half_fold_wallet_name: Element,
    pub half_fold_chain: Element,

    // Profile
    pub profile_select: HtmlSelectElement,
    pub add_profile_btn: HtmlElement,
    pub active_wallet_select: HtmlSelectElement,

    // Connect
    pub connect_wallet_address: HtmlInputElement,
    pub connect_chain: HtmlInputElement,
    pub connect_token: HtmlInputElement,
    pub challenge_btn: HtmlElement,
    pub verify_btn: HtmlElement,
    pub bind_wallet_btn: HtmlElement,
    pub connect_result: Element,

    // Balance
    pub balance_wallet_address: HtmlInputElement,
    pub balance_chain: HtmlInputElement,
    pub balance_asset: HtmlSelectElement,
    pub balance_network_icon: HtmlImageElement,
    pub balance_coin_icon: HtmlImageElement,
    pub balance_btn: HtmlElement,
    pub balance_result: Element,

    // Sign
    pub sign_wallet_address: HtmlInputElement,
    pub sign_purpose: HtmlSelectElement,
    pub sign_payload: HtmlTextAreaElement,
    pub sign_btn: HtmlElement,
    pub sign_result: Element,

    // Transfer / Submit
    pub submit_from: HtmlInputElement,
    pub submit_to: HtmlInputElement,
    pub submit_amount: HtmlInputElement,
    pub submit_asset: HtmlSelectElement,
    pub submit_chain: HtmlInputElement,
    pub submit_nonce: HtmlInputElement,
    pub nonce_btn: HtmlElement,
    pub nonce_display: Element,
    pub submit_tx_btn: HtmlElement,
    pub submit_result: Element,

    // History
    pub tx_hash: HtmlInputElement,
    pub tx_status_btn: HtmlElement,
    pub history_result: Element,

    // Platform integration
    pub chain_config_btn: HtmlElement,
    pub chain_config_result: Element,
    pub fd_wallet_address: HtmlInputElement,
    pub wallet_status_btn: HtmlElement,
    pub wallet_status_result: Element,
    pub pc_wallet_address: HtmlInputElement,
    pub pc_challenge: HtmlInputElement,
    pub pc_tx_hash: HtmlInputElement,
    pub commitment_btn: HtmlElement,
    pub commitment_result: Element,
    pub health_btn: HtmlElement,
    pub readyz_btn: HtmlElement,
    pub startupz_btn: HtmlElement,
    pub ops_result: Element,
}

macro_rules! get_el {
    ($id:expr) => {
        by_id($id).ok_or_else(|| JsValue::from_str(&format!("missing element #{}", $id)))?
    };
}

macro_rules! get_input {
    ($id:expr) => {
        by_id_typed::<HtmlInputElement>($id)
            .ok_or_else(|| JsValue::from_str(&format!("missing input #{}", $id)))?
    };
}

macro_rules! get_select {
    ($id:expr) => {
        by_id_typed::<HtmlSelectElement>($id)
            .ok_or_else(|| JsValue::from_str(&format!("missing select #{}", $id)))?
    };
}

macro_rules! get_textarea {
    ($id:expr) => {
        by_id_typed::<HtmlTextAreaElement>($id)
            .ok_or_else(|| JsValue::from_str(&format!("missing textarea #{}", $id)))?
    };
}

macro_rules! get_img {
    ($id:expr) => {
        by_id_typed::<HtmlImageElement>($id)
            .ok_or_else(|| JsValue::from_str(&format!("missing img #{}", $id)))?
    };
}

macro_rules! get_html {
    ($id:expr) => {
        by_id_typed::<HtmlElement>($id)
            .ok_or_else(|| JsValue::from_str(&format!("missing html element #{}", $id)))?
    };
}

impl Elements {
    /// Resolve all DOM references. Call once after DOMContentLoaded.
    pub fn bind() -> Result<Elements, JsValue> {
        Ok(Elements {
            wallet_window: query(".wallet-window")
                .ok_or_else(|| JsValue::from_str("missing .wallet-window"))?,
            wallet_folded: get_el!("walletFolded"),
            wallet_fold_toggle: get_el!("walletFoldToggle"),
            wallet_app: get_el!("walletApp"),

            base_url: get_input!("baseUrl"),
            skin_select: get_select!("skinSelect"),
            form_select: get_select!("formSelect"),
            skin_cycle_btn: get_html!("skinCycleBtn"),

            tabs: query_all(".tab"),
            panels: query_all(".panel"),

            create_wallet_btn: get_html!("createWalletBtn"),
            create_result: get_el!("createResult"),
            wallet_label_input: get_input!("walletLabelInput"),
            wallet_passphrase_input: get_input!("walletPassphraseInput"),
            restore_wallet_btn: get_html!("restoreWalletBtn"),
            restore_hint: get_el!("restoreHint"),
            refresh_wallets_btn: get_html!("refreshWalletsBtn"),

            wallet_list_container: get_el!("walletListContainer"),

            half_fold_wallet_name: get_el!("halfFoldWalletName"),
            half_fold_chain: get_el!("halfFoldChain"),

            profile_select: get_select!("profileSelect"),
            add_profile_btn: get_html!("addProfileBtn"),
            active_wallet_select: get_select!("activeWalletSelect"),

            connect_wallet_address: get_input!("connectWalletAddress"),
            connect_chain: get_input!("connectChain"),
            connect_token: get_input!("connectToken"),
            challenge_btn: get_html!("challengeBtn"),
            verify_btn: get_html!("verifyBtn"),
            bind_wallet_btn: get_html!("bindWalletBtn"),
            connect_result: get_el!("connectResult"),

            balance_wallet_address: get_input!("balanceWalletAddress"),
            balance_chain: get_input!("balanceChain"),
            balance_asset: get_select!("balanceAsset"),
            balance_network_icon: get_img!("balanceNetworkIcon"),
            balance_coin_icon: get_img!("balanceCoinIcon"),
            balance_btn: get_html!("balanceBtn"),
            balance_result: get_el!("balanceResult"),

            sign_wallet_address: get_input!("signWalletAddress"),
            sign_purpose: get_select!("signPurpose"),
            sign_payload: get_textarea!("signPayload"),
            sign_btn: get_html!("signBtn"),
            sign_result: get_el!("signResult"),

            submit_from: get_input!("submitFrom"),
            submit_to: get_input!("submitTo"),
            submit_amount: get_input!("submitAmount"),
            submit_asset: get_select!("submitAsset"),
            submit_chain: get_input!("submitChain"),
            submit_nonce: get_input!("submitNonce"),
            nonce_btn: get_html!("nonceBtn"),
            nonce_display: get_el!("nonceDisplay"),
            submit_tx_btn: get_html!("submitTxBtn"),
            submit_result: get_el!("submitResult"),

            tx_hash: get_input!("txHash"),
            tx_status_btn: get_html!("txStatusBtn"),
            history_result: get_el!("historyResult"),

            chain_config_btn: get_html!("chainConfigBtn"),
            chain_config_result: get_el!("chainConfigResult"),
            fd_wallet_address: get_input!("fdWalletAddress"),
            wallet_status_btn: get_html!("walletStatusBtn"),
            wallet_status_result: get_el!("walletStatusResult"),
            pc_wallet_address: get_input!("pcWalletAddress"),
            pc_challenge: get_input!("pcChallenge"),
            pc_tx_hash: get_input!("pcTxHash"),
            commitment_btn: get_html!("commitmentBtn"),
            commitment_result: get_el!("commitmentResult"),
            health_btn: get_html!("healthBtn"),
            readyz_btn: get_html!("readyzBtn"),
            startupz_btn: get_html!("startupzBtn"),
            ops_result: get_el!("opsResult"),
        })
    }
}
