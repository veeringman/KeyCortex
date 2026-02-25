use anyhow::{Result, anyhow};
use kc_api_types::{AuthChallengeResponse, AuthVerifyResponse};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PendingChallenge {
    pub challenge: String,
    pub expires_in_seconds: u64,
}

pub fn issue_challenge(expires_in_seconds: u64) -> PendingChallenge {
    PendingChallenge {
        challenge: Uuid::new_v4().to_string(),
        expires_in_seconds,
    }
}

pub fn challenge_response(challenge: &PendingChallenge) -> AuthChallengeResponse {
    AuthChallengeResponse {
        challenge: challenge.challenge.clone(),
        expires_in: challenge.expires_in_seconds,
    }
}

pub fn verify_signature_placeholder(
    wallet_address: &str,
    challenge: &str,
    signature: &str,
) -> Result<AuthVerifyResponse> {
    if wallet_address.trim().is_empty() || challenge.trim().is_empty() || signature.trim().is_empty() {
        return Err(anyhow!("wallet_address, challenge, and signature are required"));
    }

    let verified_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| anyhow!("time error: {err}"))?
        .as_millis();

    Ok(AuthVerifyResponse {
        valid: true,
        wallet_address: wallet_address.to_owned(),
        verified_at_epoch_ms,
    })
}
