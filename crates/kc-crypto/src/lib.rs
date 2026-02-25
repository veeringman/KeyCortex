use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey, Verifier};
#[cfg(feature = "secp256k1")]
use k256::ecdsa::{
    Signature as Secp256k1Signature, SigningKey as Secp256k1SigningKey,
    VerifyingKey as Secp256k1VerifyingKey,
    signature::{Signer as K256Signer, Verifier as K256Verifier},
};
use kc_api_types::SignPurpose;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

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

    pub fn from_secret_key_bytes(mut secret_key: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&secret_key);
        secret_key.zeroize();
        Self {
            signing_key,
        }
    }

    /// Derive a deterministic Ed25519 keypair from a passphrase.
    /// Uses domain-tagged PBKDF-style SHA-256 derivation (1000 rounds).
    /// The same passphrase always produces the same wallet address.
    pub fn from_passphrase(passphrase: &str) -> Self {
        let mut seed = [0u8; 32];
        // Initial hash: domain-tagged passphrase
        let mut hasher = Sha256::new();
        hasher.update(b"keycortex:wallet-derive:v1:");
        hasher.update(passphrase.as_bytes());
        let digest = hasher.finalize();
        seed.copy_from_slice(&digest);

        // Stretch with 1000 rounds
        for _ in 0..1000 {
            let mut h = Sha256::new();
            h.update(b"keycortex:stretch:");
            h.update(seed);
            let d = h.finalize();
            seed.copy_from_slice(&d);
        }

        let signing_key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        Self { signing_key }
    }

    pub fn verify(&self, payload: &[u8], purpose: SignPurpose, signature: &[u8]) -> Result<bool> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        if signature.len() != 64 {
            return Err(anyhow!("invalid ed25519 signature length"));
        }

        let signing_input = signing_input(payload, purpose);
        let signature = Signature::from_slice(signature)
            .map_err(|_| anyhow!("invalid ed25519 signature format"))?;

        Ok(self
            .signing_key
            .verifying_key()
            .verify(&signing_input, &signature)
            .is_ok())
    }
}

#[cfg(feature = "secp256k1")]
pub struct Secp256k1Signer {
    signing_key: Secp256k1SigningKey,
}

#[cfg(feature = "secp256k1")]
impl Secp256k1Signer {
    pub fn new_random() -> Self {
        let mut rng = OsRng;
        let signing_key = Secp256k1SigningKey::random(&mut rng);
        Self { signing_key }
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        let verifying_key = self.signing_key.verifying_key();
        verifying_key.to_encoded_point(true).as_bytes().to_vec()
    }

    pub fn public_key_hex(&self) -> String {
        to_hex(&self.public_key_bytes())
    }

    pub fn wallet_address(&self) -> String {
        let digest = Sha256::digest(self.public_key_bytes());
        format!("0x{}", to_hex(&digest[..20]))
    }

    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes().into()
    }

    pub fn from_secret_key_bytes(mut secret_key: [u8; 32]) -> Result<Self> {
        let signing_key = Secp256k1SigningKey::from_bytes((&secret_key).into())
            .map_err(|_| anyhow!("invalid secp256k1 secret key"))?;
        secret_key.zeroize();
        Ok(Self { signing_key })
    }

    pub fn verify(&self, payload: &[u8], purpose: SignPurpose, signature: &[u8]) -> Result<bool> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        if signature.len() != 64 {
            return Err(anyhow!("invalid secp256k1 signature length"));
        }

        let signing_input = signing_input(payload, purpose);
        let parsed = Secp256k1Signature::try_from(signature)
            .map_err(|_| anyhow!("invalid secp256k1 signature format"))?;
        let verifying_key: Secp256k1VerifyingKey = *self.signing_key.verifying_key();

        Ok(verifying_key.verify(&signing_input, &parsed).is_ok())
    }
}

#[cfg(feature = "secp256k1")]
impl Signer for Secp256k1Signer {
    fn sign(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        let signing_input = signing_input(payload, purpose);
        let signature: Secp256k1Signature = self.signing_key.sign(&signing_input);
        Ok(signature.to_bytes().to_vec())
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, payload: &[u8], purpose: SignPurpose) -> Result<Vec<u8>> {
        if payload.is_empty() {
            return Err(anyhow!("payload cannot be empty"));
        }

        let signing_input = signing_input(payload, purpose);

        let signature: Signature = self.signing_key.sign(&signing_input);
        Ok(signature.to_bytes().to_vec())
    }
}

fn signing_input(payload: &[u8], purpose: SignPurpose) -> Vec<u8> {
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
    signing_input
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

    let mut key_stream = derive_key_stream(encryption_key, secret_key.len());
    let mut encrypted = Vec::with_capacity(secret_key.len());
    for (index, byte) in secret_key.iter().enumerate() {
        encrypted.push(byte ^ key_stream[index]);
    }
    key_stream.zeroize();
    Ok(encrypted)
}

pub fn decrypt_key_material(encrypted: &[u8], encryption_key: &str) -> Result<[u8; 32]> {
    if encryption_key.trim().is_empty() {
        return Err(anyhow!("encryption key cannot be empty"));
    }

    if encrypted.len() != 32 {
        return Err(anyhow!("invalid encrypted key length"));
    }

    let mut key_stream = derive_key_stream(encryption_key, encrypted.len());
    let mut decrypted = [0_u8; 32];

    for (index, byte) in encrypted.iter().enumerate() {
        decrypted[index] = byte ^ key_stream[index];
    }

    key_stream.zeroize();

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_sign_verify_roundtrip() {
        let signer = Ed25519Signer::new_random();
        let payload = b"test-payload";
        let signature = signer
            .sign(payload, SignPurpose::Transaction)
            .expect("sign should succeed");
        let valid = signer
            .verify(payload, SignPurpose::Transaction, &signature)
            .expect("verify should succeed");
        assert!(valid);
    }

    #[cfg(feature = "secp256k1")]
    #[test]
    fn secp256k1_sign_verify_roundtrip() {
        let signer = Secp256k1Signer::new_random();
        let payload = b"test-payload";
        let signature = signer
            .sign(payload, SignPurpose::Proof)
            .expect("sign should succeed");
        let valid = signer
            .verify(payload, SignPurpose::Proof, &signature)
            .expect("verify should succeed");
        assert!(valid);
    }
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
