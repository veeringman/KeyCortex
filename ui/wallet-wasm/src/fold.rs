//! Wallet fold state machine.
//!
//! States: `folded` → `half` → `unfolded`.
//! Manages auto-fold timers upon inactivity.
//! The fold toggle uses single-click (fold) and double-click (unfold) in half state.
//!
//! **Must** match the JS `setWalletState()` exactly: classes on `.wallet-window`
//! (`folded`, `half-folded`, `unfolded`) **and** on `#walletFolded` overlay
//! (`closed`, `half`, `open`) **and** inline styles on `#walletApp`.

use crate::dom::{self, Elements};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ── Fold state ──

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldState {
    Folded,
    Half,
    Unfolded,
}

thread_local! {
    static FOLD_STATE: RefCell<FoldState> = RefCell::new(FoldState::Folded);
    static AUTO_FOLD_TIMER: RefCell<Option<i32>> = RefCell::new(None);
    static AUTO_CLOSE_TIMER: RefCell<Option<i32>> = RefCell::new(None);
    static CLICK_TIMER: RefCell<Option<i32>> = RefCell::new(None);
}

pub fn current() -> FoldState {
    FOLD_STATE.with(|s| *s.borrow())
}

fn set_state(s: FoldState) {
    FOLD_STATE.with(|st| *st.borrow_mut() = s);
}

// ── Auto-fold timers ──

fn clear_timers() {
    let w = web_sys::window().unwrap();
    AUTO_FOLD_TIMER.with(|t| {
        if let Some(id) = t.borrow_mut().take() {
            w.clear_timeout_with_handle(id);
        }
    });
    AUTO_CLOSE_TIMER.with(|t| {
        if let Some(id) = t.borrow_mut().take() {
            w.clear_timeout_with_handle(id);
        }
    });
}

/// Reset auto-fold timers (call on any user interaction).
/// Mirrors the JS `resetAutoFoldTimer()` exactly.
pub fn reset_auto_fold_timer(els: &Elements) {
    clear_timers();
    let state = current();
    if state == FoldState::Unfolded {
        // Half-fold after 30s of inactivity
        let els2 = els.clone();
        let cb = Closure::once(move || {
            if current() == FoldState::Unfolded {
                set_wallet_state(&els2, FoldState::Half);
            }
        });
        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                30_000,
            )
            .unwrap();
        AUTO_FOLD_TIMER.with(|t| *t.borrow_mut() = Some(id));
        cb.forget();

        // Fully close after 120s of inactivity (independent timer)
        let els3 = els.clone();
        let cb2 = Closure::once(move || {
            if current() != FoldState::Folded {
                set_wallet_state(&els3, FoldState::Folded);
            }
        });
        let id2 = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                cb2.as_ref().unchecked_ref(),
                120_000,
            )
            .unwrap();
        AUTO_CLOSE_TIMER.with(|t| *t.borrow_mut() = Some(id2));
        cb2.forget();
    } else if state == FoldState::Half {
        // If already half-folded, fully close after 90s more
        let els2 = els.clone();
        let cb = Closure::once(move || {
            if current() == FoldState::Half {
                set_wallet_state(&els2, FoldState::Folded);
            }
        });
        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                90_000,
            )
            .unwrap();
        AUTO_CLOSE_TIMER.with(|t| *t.borrow_mut() = Some(id));
        cb.forget();
    }
}

/// Transition the wallet to a new fold state, updating DOM classes and inline
/// styles to **exactly** match the JS `setWalletState()`.
pub fn set_wallet_state(els: &Elements, target: FoldState) {
    set_state(target);
    let win = &els.wallet_window;
    let overlay = &els.wallet_folded;
    let app = &els.wallet_app;

    // Remove all state classes (wallet-window)
    dom::remove_class(win, "folded");
    dom::remove_class(win, "half-folded");
    dom::remove_class(win, "unfolded");
    // Remove all state classes (overlay)
    dom::remove_class(overlay, "closed");
    dom::remove_class(overlay, "half");
    dom::remove_class(overlay, "open");

    // Get app element as HtmlElement for inline style access
    let app_html: &web_sys::HtmlElement = app.unchecked_ref();
    let style = app_html.style();

    match target {
        FoldState::Folded => {
            dom::add_class(win, "folded");
            dom::add_class(overlay, "closed");
            let _ = style.set_property("visibility", "hidden");
            let _ = style.set_property("opacity", "0");
            let _ = style.set_property("pointer-events", "none");
        }
        FoldState::Half => {
            dom::add_class(win, "half-folded");
            dom::add_class(overlay, "half");
            let _ = style.set_property("visibility", "visible");
            let _ = style.set_property("opacity", "1");
            let _ = style.set_property("pointer-events", "auto");
            // Compute --half-fold-height from upper half + flap
            if let Some(upper) = dom::query(".wallet-upper-half") {
                if let Some(flap) = dom::query(".wallet-flap") {
                    let upper_h: web_sys::HtmlElement = upper.unchecked_into();
                    let flap_h: web_sys::HtmlElement = flap.unchecked_into();
                    let h = upper_h.offset_height() + flap_h.offset_height();
                    let _ = overlay
                        .unchecked_ref::<web_sys::HtmlElement>()
                        .style()
                        .set_property("--half-fold-height", &format!("{}px", h));
                }
            }
        }
        FoldState::Unfolded => {
            dom::add_class(win, "unfolded");
            dom::add_class(overlay, "open");
            let _ = style.set_property("visibility", "visible");
            let _ = style.set_property("opacity", "1");
            let _ = style.set_property("pointer-events", "auto");
        }
    }
    reset_auto_fold_timer(els);
}

/// Wire click / double-click behaviour on the fold toggle.
///
/// - **Folded**: click → half
/// - **Half**: single click → fold, double click → unfold (280 ms debounce)
/// - **Unfolded**: click → half
pub fn bind_fold_toggle(els: &Elements) {
    let els_click = els.clone();
    let on_click = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        let state = current();
        match state {
            FoldState::Folded => {
                set_wallet_state(&els_click, FoldState::Half);
            }
            FoldState::Half => {
                // Debounce: wait 280 ms for possible second click
                let els2 = els_click.clone();
                let cb = Closure::once(move || {
                    // If still half after 280ms (no dblclick), fold
                    if current() == FoldState::Half {
                        set_wallet_state(&els2, FoldState::Folded);
                    }
                });
                let id = web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        280,
                    )
                    .unwrap();
                CLICK_TIMER.with(|t| *t.borrow_mut() = Some(id));
                cb.forget();
            }
            FoldState::Unfolded => {
                set_wallet_state(&els_click, FoldState::Half);
                reset_auto_fold_timer(&els_click);
            }
        }
    }) as Box<dyn FnMut(_)>);

    els.wallet_fold_toggle
        .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
        .unwrap();
    on_click.forget();

    // Double-click on fold toggle in half state → unfold
    let els_dbl = els.clone();
    let on_dblclick = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        // Cancel pending single-click timer
        CLICK_TIMER.with(|t| {
            if let Some(id) = t.borrow_mut().take() {
                web_sys::window().unwrap().clear_timeout_with_handle(id);
            }
        });
        if current() == FoldState::Half {
            set_wallet_state(&els_dbl, FoldState::Unfolded);
        }
    }) as Box<dyn FnMut(_)>);

    els.wallet_fold_toggle
        .add_event_listener_with_callback("dblclick", on_dblclick.as_ref().unchecked_ref())
        .unwrap();
    on_dblclick.forget();
}

/// Wire click on the walletFolded overlay (not the toggle button itself).
/// Mirrors JS:
///   walletFolded.addEventListener("click", (e) => {
///     if (e.target === walletFolded || e.target === walletFoldToggle) return;
///     if folded → half, if half → unfolded
///   });
pub fn bind_overlay_click(els: &Elements) {
    let els2 = els.clone();
    let folded_id = els.wallet_folded.id();
    let toggle_id = els.wallet_fold_toggle.id();
    let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
        // Only react to clicks on children inside the overlay, not overlay/toggle itself
        if let Some(target) = e.target() {
            if let Ok(target_el) = target.dyn_into::<web_sys::Element>() {
                let tid = target_el.id();
                if tid == folded_id || tid == toggle_id {
                    return;
                }
            }
        }
        match current() {
            FoldState::Folded => set_wallet_state(&els2, FoldState::Half),
            FoldState::Half => set_wallet_state(&els2, FoldState::Unfolded),
            _ => {}
        }
    }) as Box<dyn FnMut(_)>);
    els.wallet_folded
        .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
        .unwrap();
    cb.forget();
}

/// Wire brand-logo click (in header) → fold back to half.
pub fn bind_brand_logo_fold(els: &Elements) {
    if let Some(logo) = dom::query(".brand-logo") {
        // Set cursor to pointer (matches JS)
        if let Ok(html) = logo.clone().dyn_into::<web_sys::HtmlElement>() {
            let _ = html.style().set_property("cursor", "pointer");
        }
        let els2 = els.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            if current() == FoldState::Unfolded {
                set_wallet_state(&els2, FoldState::Half);
            }
        }) as Box<dyn FnMut(_)>);
        logo.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }
}
