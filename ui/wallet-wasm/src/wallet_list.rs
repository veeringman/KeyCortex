//! Wallet list rendering and selection.
//!
//! Renders wallet cards, handles assign/unassign/select,
//! and manages the wallet selector dropdown.

use crate::api;
use crate::dom::{self, Elements};
use crate::profile;
use crate::state;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Fetch wallet list from backend and re-render.
pub async fn load_wallet_list(els: &Elements) {
    let wallets = match api::request("/wallet/list", "GET", None).await {
        Ok(result) => {
            if let Some(arr) = result.get("wallets") {
                serde_json::from_value::<Vec<state::WalletInfo>>(arr.clone()).unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    };
    state::set_wallets(wallets);
    render_wallet_list(els);
    render_wallet_selector(els);

    // Highlight "New Wallet" only when no wallets exist
    let count = state::wallets().len();
    if count == 0 {
        dom::add_class(els.create_wallet_btn.unchecked_ref(), "primary");
        dom::remove_class(els.create_wallet_btn.unchecked_ref(), "secondary");
    } else {
        dom::remove_class(els.create_wallet_btn.unchecked_ref(), "primary");
        dom::add_class(els.create_wallet_btn.unchecked_ref(), "secondary");
    }
}

/// Render wallet cards in the list container.
pub fn render_wallet_list(els: &Elements) {
    let container = &els.wallet_list_container;
    dom::set_inner_html(container, "");

    let active_profile = state::active_profile().unwrap_or_default();
    let (assigned, unassigned) = profile::get_wallets_for_profile(&active_profile);
    let all: Vec<&state::WalletInfo> = assigned.iter().chain(unassigned.iter()).collect();

    if all.is_empty() {
        dom::set_inner_html(
            container,
            r#"<div class="wallet-card wallet-card--empty">No wallets yet. Create one below.</div>"#,
        );
        return;
    }

    let active_wallet = state::active_wallet().unwrap_or_default();

    for w in &all {
        let is_assigned = assigned.iter().any(|a| a.wallet_address == w.wallet_address);
        let is_active = w.wallet_address == active_wallet;

        let card = dom::create_element("div");
        let mut cls = "wallet-card".to_string();
        if is_active {
            cls.push_str(" wallet-card--active");
        }
        card.set_attribute("class", &cls).unwrap();

        let short_addr = shorten(&w.wallet_address, 8, 6);
        let label_html = match &w.label {
            Some(l) if !l.is_empty() => {
                format!(r#"<div class="wc-label" title="Click to rename">{}</div>"#, l)
            }
            _ => r#"<div class="wc-label wc-label--empty" title="Click to name">unnamed</div>"#
                .to_string(),
        };
        let user_label = match &w.bound_user_id {
            Some(u) if !u.is_empty() => format!(r#"<span class="wc-user">{}</span>"#, u),
            _ => String::new(),
        };
        let profile_name = profile::get_profile_name(&active_profile);
        let profile_label = if is_assigned {
            format!(
                r#"<span class="wc-profile wc-profile--mine">✓ {}</span>"#,
                profile_name
            )
        } else {
            r#"<span class="wc-profile wc-profile--none">unassigned</span>"#.to_string()
        };
        let pk_html = match &w.public_key {
            Some(pk) if !pk.is_empty() => {
                let short_pk = shorten(pk, 8, 6);
                format!(
                    r#"<div class="wc-pubkey" title="{}">pk: {}</div>"#,
                    pk, short_pk
                )
            }
            _ => String::new(),
        };
        let assign_btn = if is_assigned {
            format!(
                r#"<button class="wc-unassign-btn icon-btn" data-addr="{}" title="Remove from profile">&minus;</button>"#,
                w.wallet_address
            )
        } else {
            format!(
                r#"<button class="wc-assign-btn icon-btn" data-addr="{}" title="Assign to profile">&plus;</button>"#,
                w.wallet_address
            )
        };

        let html = format!(
            r#"
            {}
            <div class="wc-address" title="{}">{}</div>
            <div class="wc-meta">{} {} {}</div>
            {}
            <div class="wc-actions">
              <button class="wc-select-btn secondary" data-addr="{}">Use</button>
              <button class="wc-rename-btn icon-btn" data-addr="{}" title="Rename">✎</button>
              {}
            </div>
            "#,
            label_html,
            w.wallet_address,
            short_addr,
            w.chain,
            user_label,
            profile_label,
            pk_html,
            w.wallet_address,
            w.wallet_address,
            assign_btn,
        );

        dom::set_inner_html(&card, &html);
        container.append_child(&card).unwrap();
    }

    // Wire card buttons
    wire_wallet_card_events(els);
}

/// Wire click events on dynamically-created wallet card buttons.
fn wire_wallet_card_events(els: &Elements) {
    let container = &els.wallet_list_container;

    // Select buttons
    for btn in dom::query_all_within(container, ".wc-select-btn") {
        let addr = btn
            .get_attribute("data-addr")
            .unwrap_or_default();
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            select_active_wallet(&els2, &addr);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // Rename buttons
    for btn in dom::query_all_within(container, ".wc-rename-btn") {
        let addr = btn
            .get_attribute("data-addr")
            .unwrap_or_default();
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let els3 = els2.clone();
            let a = addr.clone();
            wasm_bindgen_futures::spawn_local(async move {
                crate::wallet_ops::on_rename_wallet(&els3, &a).await;
            });
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // Label click → rename
    for lbl in dom::query_all_within(container, ".wc-label") {
        let _ = lbl
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("cursor", "pointer");
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            let target = e.current_target().unwrap();
            let el: &web_sys::Element = target.unchecked_ref();
            let card = el.closest(".wallet-card").ok().flatten();
            if let Some(card) = card {
                if let Some(rename_btn) = card.query_selector(".wc-rename-btn").ok().flatten() {
                    let addr = rename_btn.get_attribute("data-addr").unwrap_or_default();
                    let els3 = els2.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        crate::wallet_ops::on_rename_wallet(&els3, &addr).await;
                    });
                }
            }
        }) as Box<dyn FnMut(_)>);
        lbl.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // Assign buttons
    for btn in dom::query_all_within(container, ".wc-assign-btn") {
        let addr = btn
            .get_attribute("data-addr")
            .unwrap_or_default();
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let ap = state::active_profile().unwrap_or_default();
            profile::assign_wallet_to_profile(&addr, &ap);
            render_wallet_list(&els2);
            render_wallet_selector(&els2);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    // Unassign buttons
    for btn in dom::query_all_within(container, ".wc-unassign-btn") {
        let addr = btn
            .get_attribute("data-addr")
            .unwrap_or_default();
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let ap = state::active_profile().unwrap_or_default();
            profile::unassign_wallet_from_profile(&addr, &ap);
            render_wallet_list(&els2);
            render_wallet_selector(&els2);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
}

/// Render the wallet selector dropdown (with optgroups).
pub fn render_wallet_selector(els: &Elements) {
    let sel = &els.active_wallet_select;
    dom::set_inner_html(sel.unchecked_ref(), "");

    let active_profile = state::active_profile().unwrap_or_default();
    let (assigned, unassigned) = profile::get_wallets_for_profile(&active_profile);
    let all_empty = assigned.is_empty() && unassigned.is_empty();

    if all_empty {
        let opt = dom::create_option("", "\u{2014} no wallets \u{2014}", false);
        sel.append_child(&opt).unwrap();
        return;
    }

    let active_wallet = state::active_wallet().unwrap_or_default();

    if !assigned.is_empty() {
        let grp = dom::create_element("optgroup");
        grp.set_attribute("label", "My Wallets").unwrap();
        for w in &assigned {
            let short = shorten(&w.wallet_address, 8, 6);
            let text = match &w.label {
                Some(l) if !l.is_empty() => format!("{} \u{2014} {}", l, short),
                _ => {
                    let suffix = match &w.bound_user_id {
                        Some(u) if !u.is_empty() => format!(" ({})", u),
                        _ => String::new(),
                    };
                    format!("{}{}", short, suffix)
                }
            };
            let opt = dom::create_option(
                &w.wallet_address,
                &text,
                w.wallet_address == active_wallet,
            );
            grp.append_child(&opt).unwrap();
        }
        sel.append_child(&grp).unwrap();
    }

    if !unassigned.is_empty() {
        let grp = dom::create_element("optgroup");
        grp.set_attribute("label", "Unassigned").unwrap();
        for w in &unassigned {
            let short = shorten(&w.wallet_address, 8, 6);
            let text = match &w.label {
                Some(l) if !l.is_empty() => format!("{} \u{2014} {}", l, short),
                _ => short.clone(),
            };
            let opt = dom::create_option(
                &w.wallet_address,
                &text,
                w.wallet_address == active_wallet,
            );
            grp.append_child(&opt).unwrap();
        }
        sel.append_child(&grp).unwrap();
    }
}

/// Set the active wallet and populate all address fields.
pub fn select_active_wallet(els: &Elements, addr: &str) {
    state::set_active_wallet(addr);
    state::local_set("kc_active_wallet", addr);

    dom::set_select_value(&els.active_wallet_select, addr);
    els.connect_wallet_address.set_value(addr);
    els.balance_wallet_address.set_value(addr);
    els.sign_wallet_address.set_value(addr);
    els.submit_from.set_value(addr);
    els.fd_wallet_address.set_value(addr);
    els.pc_wallet_address.set_value(addr);

    update_half_fold_info(els);
    render_wallet_list(els);
}

/// Update the half-fold info bar with the active wallet details.
pub fn update_half_fold_info(els: &Elements) {
    let active = state::active_wallet().unwrap_or_default();
    let wallets = state::wallets();
    let w = wallets.iter().find(|w| w.wallet_address == active);

    match w {
        Some(w) => {
            let name = match &w.label {
                Some(l) if !l.is_empty() => l.clone(),
                _ => shorten(&w.wallet_address, 8, 6),
            };
            dom::set_text(&els.half_fold_wallet_name, &name);
            dom::set_text(&els.half_fold_chain, &w.chain);
        }
        None => {
            dom::set_text(&els.half_fold_wallet_name, "\u{2014}");
            dom::set_text(&els.half_fold_chain, "flowcortex-l1");
        }
    }
}

// ── Helpers ──

fn shorten(s: &str, head: usize, tail: usize) -> String {
    if s.len() <= head + tail + 1 {
        s.to_string()
    } else {
        format!("{}\u{2026}{}", &s[..head], &s[s.len() - tail..])
    }
}
