use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &str,
) -> Result<()> {
    // Trim any whitespace from the hex strings
    let public_key_hex = public_key_hex.trim();
    let signature_hex = signature_hex.trim();

    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|e| anyhow!("Failed to decode public key: {}", e))?;
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|e| anyhow!("Failed to decode signature: {}", e))?;

    if public_key_bytes.len() != 32 {
        return Err(anyhow!("Invalid public key length: expected 32 bytes, got {}", public_key_bytes.len()));
    }

    if signature_bytes.len() != 64 {
        return Err(anyhow!("Invalid signature length: expected 64 bytes, got {}", signature_bytes.len()));
    }

    let public_key_array: [u8; 32] = public_key_bytes.try_into().unwrap();
    let signature_array: [u8; 64] = signature_bytes.try_into().unwrap();

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)?;
    let signature = Signature::from_bytes(&signature_array);

    // Discord concatenates timestamp and body with no separator
    let message = format!("{}{}", timestamp, body);

    verifying_key.verify(message.as_bytes(), &signature)
        .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

    Ok(())
}
