use serde::{Serialize, Deserialize};

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
    signer: &impl crate::Signer,
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
    let signature = base64::encode(&signature_bytes);
    FortressDigitalContextPayload {
        signature,
        ..payload
    }
}
