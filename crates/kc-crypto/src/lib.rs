use anyhow::{Result, anyhow};
use kc_api_types::SignPurpose;

pub trait Signer: Send + Sync {
    fn sign(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>>;
}

#[derive(Default)]
pub struct PlaceholderSigner;

impl Signer for PlaceholderSigner {
    fn sign(&self, payload: &[u8], _purpose: SignPurpose) -> Result<Vec<u8>> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        Ok(payload.to_vec())
    }
}
