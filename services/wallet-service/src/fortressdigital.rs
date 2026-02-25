use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use kc_api_types::{
    FortressDigitalWalletStatusRequest, FortressDigitalWalletStatusResponse,
    WalletBindingStatus,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct FortressDigitalContextPayload {
    pub wallet_address: String,
    pub user_id: String,
    pub chain: String,
    pub session_id: String,
    pub issued_at_epoch_ms: u128,
    pub expires_at_epoch_ms: u128,
    pub context_data: String, // JSON or base64-encoded context
    pub signature: String,    // Signed by wallet-service
}

// Example function to generate and sign the payload
pub fn generate_context_payload(
    wallet_address: &str,
    user_id: &str,
    chain: &str,
    session_id: &str,
    context_data: &str,
    issued_at_epoch_ms: u128,
    expires_at_epoch_ms: u128,
    signer: &impl kc_crypto::Signer,
) -> FortressDigitalContextPayload {
    let payload = FortressDigitalContextPayload {
        wallet_address: wallet_address.to_owned(),
        user_id: user_id.to_owned(),
        chain: chain.to_owned(),
        session_id: session_id.to_owned(),
        issued_at_epoch_ms,
        expires_at_epoch_ms,
        context_data: context_data.to_owned(),
        signature: String::new(), // Placeholder
    };
    let serialized = serde_json::to_string(&payload).unwrap();
    let signature_bytes = signer
        .sign(serialized.as_bytes(), kc_api_types::SignPurpose::Proof)
        .unwrap_or_default();
    let signature = STANDARD.encode(&signature_bytes);
    FortressDigitalContextPayload {
        signature,
        ..payload
    }
}

/// Wallet verification status for FortressDigital risk scoring and policy gating.
///
/// Returns enriched wallet signals:
///   - wallet existence + binding status
///   - key type (local custody)
///   - last verification time
///   - signing frequency hint
///   - risk signals for policy engine
pub fn build_wallet_status(
    wallet_address: &str,
    chain: &str,
    wallet_exists: bool,
    binding: Option<&kc_storage::WalletBindingRecord>,
    audit_event_count: usize,
    now: u128,
) -> FortressDigitalWalletStatusResponse {
    let binding_status = if let Some(b) = binding {
        WalletBindingStatus {
            bound: true,
            user_id: Some(b.user_id.clone()),
            last_verified_epoch_ms: Some(b.last_verified_epoch_ms),
        }
    } else {
        WalletBindingStatus {
            bound: false,
            user_id: None,
            last_verified_epoch_ms: None,
        }
    };

    let last_verification = binding.map(|b| b.last_verified_epoch_ms);

    let frequency_hint = match audit_event_count {
        0 => "none",
        1..=5 => "low",
        6..=20 => "moderate",
        _ => "high",
    }
    .to_owned();

    let mut risk_signals = Vec::new();

    if !wallet_exists {
        risk_signals.push("wallet_not_found".to_owned());
    }

    if !binding_status.bound {
        risk_signals.push("wallet_not_bound".to_owned());
    }

    if let Some(last_ms) = last_verification {
        let age_hours = (now.saturating_sub(last_ms)) / (1000 * 60 * 60);
        if age_hours > 24 {
            risk_signals.push("verification_stale_24h".to_owned());
        }
    } else if wallet_exists {
        risk_signals.push("never_verified".to_owned());
    }

    FortressDigitalWalletStatusResponse {
        wallet_address: wallet_address.to_owned(),
        chain: chain.to_owned(),
        wallet_exists,
        binding_status,
        key_type: "local-ed25519".to_owned(),
        last_verification_epoch_ms: last_verification,
        signature_frequency_hint: frequency_hint,
        risk_signals,
    }
}
