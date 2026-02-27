//! HTTP API client.
//!
//! Wraps `fetch` for JSON requests to the wallet-service backend.
//! `base_url()` auto-detects Codespace forwarding.
//! Extend by adding new request helpers or auth header injection.

use crate::dom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

/// Determine the API base URL.
///
/// Priority: user-supplied `#baseUrl` input → Codespace auto-detect → same-origin `:8080`.
pub fn base_url() -> String {
    if let Some(input) = dom::by_id_typed::<web_sys::HtmlInputElement>("baseUrl") {
        let v = input.value().trim().to_string();
        if !v.is_empty() {
            return v.trim_end_matches('/').to_string();
        }
    }

    let loc = dom::window().location();
    let host = loc.hostname().unwrap_or_default();
    let protocol = loc.protocol().unwrap_or_else(|_| "http:".into());

    // GitHub Codespaces: rewrite port in hostname
    if host.contains(".app.github.dev") {
        let base = host.replace(".app.github.dev", "");
        // Strip current forwarded port prefix, replace with 8080
        let parts: Vec<&str> = base.rsplitn(2, '-').collect();
        let prefix = if parts.len() == 2 { parts[1] } else { &base };
        return format!("https://{}-8080.app.github.dev", prefix);
    }

    format!("{}//{}:8080", protocol, host)
}

/// Perform a fetch request, returning the parsed JSON as `serde_json::Value`.
pub async fn request(
    path: &str,
    method: &str,
    body: Option<String>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", base_url(), path);

    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    let headers = Headers::new().map_err(|e| format!("{:?}", e))?;

    if let Some(ref b) = body {
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| format!("{:?}", e))?;
        let js_body = JsValue::from_str(b);
        opts.set_body(&js_body);
    }

    opts.set_headers(&headers);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    let window = dom::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch error: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "response is not a Response".to_string())?;

    let text = JsFuture::from(resp.text().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("text error: {:?}", e))?;

    let text_str = text.as_string().unwrap_or_default();

    if !resp.ok() {
        return Err(format!("{} {}: {}", resp.status(), resp.status_text(), text_str));
    }

    serde_json::from_str(&text_str).map_err(|e| format!("JSON parse error: {} — raw: {}", e, text_str))
}

/// Fetch a URL and return the body as a plain string.
pub async fn fetch_text(url: &str) -> Result<String, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{:?}", e))?;

    let window = dom::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch error: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "not a Response".to_string())?;

    let text = JsFuture::from(resp.text().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("text error: {:?}", e))?;

    Ok(text.as_string().unwrap_or_default())
}

/// Base64-encode a UTF-8 string (mirrors JS `btoa`).
pub fn to_base64(input: &str) -> String {
    let window = dom::window();
    window.btoa(input).unwrap_or_default()
}

/// Write a result (JSON or error) into a `<pre>` element.
pub fn set_result(el: &web_sys::Element, value: &serde_json::Value) {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value));
    dom::remove_class(el, "error");
    el.set_text_content(Some(&pretty));
}

/// Write an error string into a `<pre>` element.
pub fn set_result_error(el: &web_sys::Element, msg: &str) {
    dom::add_class(el, "error");
    el.set_text_content(Some(msg));
}
