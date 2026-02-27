//! Wallet CRUD operations.
//!
//! Each function corresponds to a backend API call.
//! Extend by adding new operations and wiring them in `events.rs`.

use wasm_bindgen::JsCast;

use crate::api;
use crate::dom::{self, Elements};
use crate::state;
use crate::wallet_list;

/// POST /wallet/create
pub async fn on_create_wallet(els: &Elements) {
    let label = dom::get_input_value(&els.wallet_label_input);
    let passphrase = dom::get_input_value(&els.wallet_passphrase_input);

    let mut body = serde_json::json!({ "chain": "flowcortex-l1" });
    if !label.is_empty() {
        body["label"] = serde_json::Value::String(label);
    }
    if !passphrase.is_empty() {
        body["passphrase"] = serde_json::Value::String(passphrase);
    }

    match api::request("/wallet/create", "POST", Some(body.to_string())).await {
        Ok(result) => {
            api::set_result(&els.create_result, &result);
            // Auto-assign to active profile
            if let Some(addr) = result.get("wallet_address").and_then(|v| v.as_str()) {
                if let Some(profile) = state::active_profile() {
                    crate::profile::assign_wallet_to_profile(addr, &profile);
                }
                wallet_list::load_wallet_list(els).await;
                wallet_list::select_active_wallet(els, addr);
            }
        }
        Err(e) => api::set_result_error(&els.create_result, &e),
    }
}

/// POST /wallet/restore
pub async fn on_restore_wallet(els: &Elements) {
    let passphrase = dom::get_input_value(&els.wallet_passphrase_input);
    if passphrase.is_empty() {
        els.restore_hint
            .set_text_content(Some("passphrase required for restore"));
        let _ = els
            .restore_hint
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("display", "block");
        return;
    }

    let body = serde_json::json!({
        "chain": "flowcortex-l1",
        "passphrase": passphrase,
    });

    match api::request("/wallet/restore", "POST", Some(body.to_string())).await {
        Ok(result) => {
            api::set_result(&els.create_result, &result);
            wallet_list::load_wallet_list(els).await;
        }
        Err(e) => api::set_result_error(&els.create_result, &e),
    }
}

/// POST /wallet/rename (prompt for new name)
pub async fn on_rename_wallet(els: &Elements, wallet_address: &str) {
    let new_name = dom::window()
        .prompt_with_message("Enter new wallet name:")
        .ok()
        .flatten()
        .unwrap_or_default();
    if new_name.trim().is_empty() {
        return;
    }

    let body = serde_json::json!({
        "wallet_address": wallet_address,
        "label": new_name.trim(),
    });

    match api::request("/wallet/rename", "POST", Some(body.to_string())).await {
        Ok(_) => {
            wallet_list::load_wallet_list(els).await;
        }
        Err(e) => api::set_result_error(&els.create_result, &e),
    }
}

/// POST /auth/bind
pub async fn on_bind_wallet(els: &Elements) {
    let addr = dom::get_input_value(&els.connect_wallet_address);
    let chain = dom::get_input_value(&els.connect_chain);
    let token = dom::get_input_value(&els.connect_token);

    let body = serde_json::json!({
        "wallet_address": addr,
        "chain": if chain.is_empty() { "flowcortex-l1".to_string() } else { chain },
        "token": token,
    });

    match api::request("/auth/bind", "POST", Some(body.to_string())).await {
        Ok(result) => api::set_result(&els.connect_result, &result),
        Err(e) => api::set_result_error(&els.connect_result, &e),
    }
}

/// GET /wallet/balance
pub async fn on_fetch_balance(els: &Elements) {
    let addr = dom::get_input_value(&els.balance_wallet_address);
    let chain = dom::get_input_value(&els.balance_chain);
    let asset = dom::get_select_value(&els.balance_asset);

    let query = format!(
        "wallet_address={}&chain={}&asset={}",
        js_sys::encode_uri_component(&addr),
        js_sys::encode_uri_component(&if chain.is_empty() { "flowcortex-l1".into() } else { chain }),
        js_sys::encode_uri_component(&asset),
    );

    match api::request(&format!("/wallet/balance?{}", query), "GET", None).await {
        Ok(result) => api::set_result(&els.balance_result, &result),
        Err(e) => api::set_result_error(&els.balance_result, &e),
    }
}

/// POST /wallet/sign
pub async fn on_sign_payload(els: &Elements) {
    let addr = dom::get_input_value(&els.sign_wallet_address);
    let purpose = dom::get_select_value(&els.sign_purpose);
    let payload_raw = dom::get_textarea_value(&els.sign_payload);

    let body = serde_json::json!({
        "wallet_address": addr,
        "payload": api::to_base64(&payload_raw),
        "purpose": purpose,
    });

    match api::request("/wallet/sign", "POST", Some(body.to_string())).await {
        Ok(result) => api::set_result(&els.sign_result, &result),
        Err(e) => api::set_result_error(&els.sign_result, &e),
    }
}

/// GET /wallet/nonce
pub async fn on_fetch_nonce(els: &Elements) {
    let addr = dom::get_input_value(&els.submit_from);
    if addr.is_empty() {
        els.nonce_display
            .set_text_content(Some("set 'From' address first"));
        return;
    }
    let query = format!(
        "wallet_address={}",
        js_sys::encode_uri_component(&addr)
    );

    match api::request(&format!("/wallet/nonce?{}", query), "GET", None).await {
        Ok(result) => {
            let last = result
                .get("last_nonce")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let next = result
                .get("next_nonce")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            els.nonce_display
                .set_text_content(Some(&format!("last: {} · next: {}", last, next)));
            els.submit_nonce.set_value(&next.to_string());
        }
        Err(e) => {
            els.nonce_display.set_text_content(Some("\u{2014}"));
            api::set_result_error(&els.submit_result, &e);
        }
    }
}

/// POST /wallet/submit
pub async fn on_submit_tx(els: &Elements) {
    let nonce_str = dom::get_input_value(&els.submit_nonce);
    let nonce: i64 = nonce_str.parse().unwrap_or(0);
    if nonce < 1 {
        api::set_result_error(&els.submit_result, "nonce required (use Get Nonce)");
        return;
    }

    let chain_val = dom::get_input_value(&els.submit_chain);
    let body = serde_json::json!({
        "from": dom::get_input_value(&els.submit_from),
        "to": dom::get_input_value(&els.submit_to),
        "amount": dom::get_input_value(&els.submit_amount),
        "asset": dom::get_select_value(&els.submit_asset),
        "chain": if chain_val.is_empty() { "flowcortex-l1".to_string() } else { chain_val },
        "nonce": nonce,
    });

    match api::request("/wallet/submit", "POST", Some(body.to_string())).await {
        Ok(result) => {
            api::set_result(&els.submit_result, &result);
            // Populate tx hash for easy lookup
            if let Some(hash) = result.get("tx_hash").and_then(|v| v.as_str()) {
                els.tx_hash.set_value(hash);
            }
        }
        Err(e) => api::set_result_error(&els.submit_result, &e),
    }
}

/// GET /wallet/tx/:hash
pub async fn on_fetch_tx_status(els: &Elements) {
    let tx_hash = dom::get_input_value(&els.tx_hash);
    let path = format!("/wallet/tx/{}", js_sys::encode_uri_component(&tx_hash));

    match api::request(&path, "GET", None).await {
        Ok(result) => api::set_result(&els.history_result, &result),
        Err(e) => api::set_result_error(&els.history_result, &e),
    }
}

/// POST /auth/challenge
pub async fn on_challenge(els: &Elements) {
    match api::request("/auth/challenge", "POST", None).await {
        Ok(result) => {
            if let Some(c) = result.get("challenge").and_then(|v| v.as_str()) {
                state::set_last_challenge(Some(c.to_string()));
            }
            api::set_result(&els.connect_result, &result);
        }
        Err(e) => api::set_result_error(&els.connect_result, &e),
    }
}

/// Sign + verify flow
pub async fn on_verify(els: &Elements) {
    let addr = dom::get_input_value(&els.connect_wallet_address);
    if addr.is_empty() {
        api::set_result_error(&els.connect_result, "wallet address required");
        return;
    }
    let challenge = match state::last_challenge() {
        Some(c) => c,
        None => {
            api::set_result_error(&els.connect_result, "request a challenge first");
            return;
        }
    };

    // Sign the challenge
    let sign_body = serde_json::json!({
        "wallet_address": addr,
        "payload": api::to_base64(&challenge),
        "purpose": "auth",
    });

    let sign_result =
        match api::request("/wallet/sign", "POST", Some(sign_body.to_string())).await {
            Ok(r) => r,
            Err(e) => {
                api::set_result_error(&els.connect_result, &e);
                return;
            }
        };

    let signature = sign_result
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    // Verify
    let verify_body = serde_json::json!({
        "wallet_address": addr,
        "signature": signature,
        "challenge": challenge,
    });

    match api::request("/auth/verify", "POST", Some(verify_body.to_string())).await {
        Ok(result) => api::set_result(&els.connect_result, &result),
        Err(e) => api::set_result_error(&els.connect_result, &e),
    }
}
