//! Event binding.
//!
//! Wires all UI event listeners. Mirrors the JS `bindEvents()` function.
//! To add new events, add closures here and (if async) spawn via
//! `wasm_bindgen_futures::spawn_local`.

use crate::dom::{self, Elements};
use crate::fold;
use crate::icons;
use crate::platform;
use crate::profile;
use crate::state;
use crate::theme;
use crate::wallet_list;
use crate::wallet_ops;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Helper: attach async click handler to an HtmlElement.
macro_rules! on_click_async {
    ($el:expr, $els:expr, $handler:expr) => {{
        let els = $els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let els2 = els.clone();
            wasm_bindgen_futures::spawn_local(async move {
                $handler(&els2).await;
            });
        }) as Box<dyn FnMut(_)>);
        $el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }};
}

/// Helper: attach sync click handler.
macro_rules! on_click {
    ($el:expr, $cb:expr) => {{
        let cb = Closure::wrap(Box::new($cb) as Box<dyn FnMut(web_sys::MouseEvent)>);
        $el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }};
}

/// Bind all UI event listeners. Call once after init.
pub fn bind_events(els: &Elements) {
    // ── Tabs ──
    for tab in &els.tabs {
        let tab_name = tab.get_attribute("data-tab").unwrap_or_default();
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            set_active_tab(&els2, &tab_name);
        }) as Box<dyn FnMut(_)>);
        tab.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // ── Wallet operations ──
    on_click_async!(els.create_wallet_btn, els, wallet_ops::on_create_wallet);
    on_click_async!(els.refresh_wallets_btn, els, wallet_list::load_wallet_list);
    on_click_async!(els.restore_wallet_btn, els, wallet_ops::on_restore_wallet);

    // ── Profile ──
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let els3 = els2.clone();
            wasm_bindgen_futures::spawn_local(async move {
                profile::on_profile_change(&els3).await;
            });
        }) as Box<dyn FnMut(_)>);
        els.profile_select
            .add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        on_click!(els.add_profile_btn, move |_: web_sys::MouseEvent| {
            profile::on_add_profile(&els2);
        });
    }

    // ── Wallet selector ──
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let addr = dom::get_select_value(&els2.active_wallet_select);
            if !addr.is_empty() {
                wallet_list::select_active_wallet(&els2, &addr);
            }
        }) as Box<dyn FnMut(_)>);
        els.active_wallet_select
            .add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // ── Connect ──
    on_click_async!(els.challenge_btn, els, wallet_ops::on_challenge);
    on_click_async!(els.verify_btn, els, wallet_ops::on_verify);
    on_click_async!(els.bind_wallet_btn, els, wallet_ops::on_bind_wallet);

    // ── Balance ──
    on_click_async!(els.balance_btn, els, wallet_ops::on_fetch_balance);

    // ── Sign ──
    on_click_async!(els.sign_btn, els, wallet_ops::on_sign_payload);

    // ── Transfer ──
    on_click_async!(els.nonce_btn, els, wallet_ops::on_fetch_nonce);
    on_click_async!(els.submit_tx_btn, els, wallet_ops::on_submit_tx);

    // ── History ──
    on_click_async!(els.tx_status_btn, els, wallet_ops::on_fetch_tx_status);

    // ── Platform ──
    on_click_async!(els.chain_config_btn, els, platform::on_chain_config);
    on_click_async!(els.wallet_status_btn, els, platform::on_wallet_status);
    on_click_async!(els.commitment_btn, els, platform::on_commitment);
    on_click_async!(els.health_btn, els, platform::on_ops_health);
    on_click_async!(els.readyz_btn, els, platform::on_ops_readyz);
    on_click_async!(els.startupz_btn, els, platform::on_ops_startupz);

    // ── Balance icons ──
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            icons::update_balance_icons(&els2);
        }) as Box<dyn FnMut(_)>);
        els.balance_asset
            .add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            icons::update_balance_icons(&els2);
        }) as Box<dyn FnMut(_)>);
        els.balance_chain
            .add_event_listener_with_callback("input", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // ── Skin / Form ──
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let els3 = els2.clone();
            let skin = dom::get_select_value(&els3.skin_select);
            wasm_bindgen_futures::spawn_local(async move {
                theme::apply_skin(&els3, &skin).await;
            });
            state::local_set("kc_wallet_skin", &dom::get_select_value(&els2.skin_select));
        }) as Box<dyn FnMut(_)>);
        els.skin_select
            .add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let form = dom::get_select_value(&els2.form_select);
            theme::apply_form(&els2, &form);
            state::local_set("kc_wallet_form", &form);
        }) as Box<dyn FnMut(_)>);
        els.form_select
            .add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let els3 = els2.clone();
            wasm_bindgen_futures::spawn_local(async move {
                theme::cycle_skin(&els3).await;
            });
        }) as Box<dyn FnMut(_)>);
        els.skin_cycle_btn
            .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // ── Fold interactions ──
    fold::bind_fold_toggle(els);
    fold::bind_overlay_click(els);
    fold::bind_brand_logo_fold(els);

    // ── Auto-fold reset on any interaction in wallet app (click, input, focus) ──
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            fold::reset_auto_fold_timer(&els2);
        }) as Box<dyn FnMut(_)>);
        els.wallet_app
            .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            fold::reset_auto_fold_timer(&els2);
        }) as Box<dyn FnMut(_)>);
        els.wallet_app
            .add_event_listener_with_callback("input", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
    {
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            fold::reset_auto_fold_timer(&els2);
        }) as Box<dyn FnMut(_)>);
        // Use capturing phase for focus, matching JS
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_capture(true);
        els.wallet_app
            .add_event_listener_with_callback_and_add_event_listener_options(
                "focus",
                cb.as_ref().unchecked_ref(),
                &opts,
            )
            .unwrap();
        cb.forget();
    }
}

/// Switch active tab and panel.
fn set_active_tab(els: &Elements, tab_name: &str) {
    for tab in &els.tabs {
        dom::toggle_class(tab, "active", tab.get_attribute("data-tab").as_deref() == Some(tab_name));
    }
    for panel in &els.panels {
        let id = panel.id();
        dom::toggle_class(panel, "active", id == tab_name);
    }
}
