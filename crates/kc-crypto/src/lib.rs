use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey};
use kc_api_types::SignPurpose;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

pub trait Signer: Send + Sync {
    fn sign(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>>;
}

pub struct Ed25519Signer {
    signing_key: SigningKey,
}

impl Ed25519Signer {
    pub fn new_random() -> Self {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        Self { signing_key }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn public_key_hex(&self) -> String {
        to_hex(&self.public_key_bytes())
    }

    pub fn wallet_address(&self) -> String {
        let digest = Sha256::digest(self.public_key_bytes());
        format!("0x{}", to_hex(&digest[..20]))
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        let purpose_tag = match purpose {
            SignPurpose::Transaction => "transaction",
            SignPurpose::Auth => "auth",
            SignPurpose::Proof => "proof",
        };

        let mut signing_input = Vec::with_capacity(32 + payload.len());
        signing_input.extend_from_slice(b"keycortex:v1:");
        signing_input.extend_from_slice(purpose_tag.as_bytes());
        signing_input.extend_from_slice(b":");
        signing_input.extend_from_slice(payload);

        let signature: Signature = self.signing_key.sign(&signing_input);
        Ok(signature.to_bytes().to_vec())
    }
}

fn to_hex(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
