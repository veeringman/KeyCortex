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

    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn from_secret_key_bytes(secret_key: [u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&secret_key),
        }
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

pub fn encrypt_key_material(secret_key: &[u8; 32], encryption_key: &str) -> Result<Vec<u8>> {
    if encryption_key.trim().is_empty() {
        return Err(anyhow!("encryption key cannot be empty"));
    }

    let key_stream = derive_key_stream(encryption_key, secret_key.len());
    let mut encrypted = Vec::with_capacity(secret_key.len());
    for (index, byte) in secret_key.iter().enumerate() {
        encrypted.push(byte ^ key_stream[index]);
    }
    Ok(encrypted)
}

pub fn decrypt_key_material(encrypted: &[u8], encryption_key: &str) -> Result<[u8; 32]> {
    if encryption_key.trim().is_empty() {
        return Err(anyhow!("encryption key cannot be empty"));
    }

    if encrypted.len() != 32 {
        return Err(anyhow!("invalid encrypted key length"));
    }

    let key_stream = derive_key_stream(encryption_key, encrypted.len());
    let mut decrypted = [0_u8; 32];

    for (index, byte) in encrypted.iter().enumerate() {
        decrypted[index] = byte ^ key_stream[index];
    }

    Ok(decrypted)
}

fn derive_key_stream(seed: &str, len: usize) -> Vec<u8> {
    let mut stream = Vec::with_capacity(len);
    let mut counter: u64 = 0;
    while stream.len() < len {
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        hasher.update(counter.to_le_bytes());
        let block = hasher.finalize();
        for byte in block {
            if stream.len() == len {
                break;
            }
            stream.push(byte);
        }
        counter += 1;
    }
    stream
}
