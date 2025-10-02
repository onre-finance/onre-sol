/// Parsed components of an Ed25519 signature verification instruction
///
/// Contains the extracted signature count, public key, and message data
/// from a Solana Ed25519 instruction for cryptographic verification.
pub struct ParsedEd25519 {
    /// Number of signatures in the instruction (must be 1 for single signature verification)
    pub sig_count: u8,
    /// 32-byte Ed25519 public key used for signature verification
    pub pubkey: [u8; 32],
    /// Message bytes that were signed
    pub message: Vec<u8>,
}

/// Parse Ed25519 verify instruction data into useful parts.
///
/// Expected data format (Solana Ed25519 instruction format):
/// ```
/// Bytes 0:     Number of signatures (u8) - must be 1
/// Bytes 1:     Instruction index (u8)
/// Bytes 2-3:   Signature offset (u16 little-endian)
/// Bytes 4-5:   Signature instruction index (u16 little-endian)
/// Bytes 6-7:   Public key offset (u16 little-endian)
/// Bytes 8-9:   Public key instruction index (u16 little-endian)
/// Bytes 10-11: Message data offset (u16 little-endian)
/// Bytes 12-13: Message data size (u16 little-endian)
/// Bytes 14-15: Message instruction index (u16 little-endian)
///
/// Variable data section:
/// - 64-byte Ed25519 signature at signature_offset
/// - 32-byte Ed25519 public key at pubkey_offset
/// - Message bytes (length = message_size) at message_offset
/// ```
///
/// Returns None if data is malformed or doesn't follow expected format.
pub fn parse_ed25519_ix(data: &[u8]) -> Option<ParsedEd25519> {
    if data.len() < 16 {
        return None;
    }
    let sig_count = data[0];
    if sig_count != 1 {
        return None; // extend if you want batching
    }

    // read offsets from header
    let sig_offset = u16::from_le_bytes([data[2], data[3]]) as usize;
    let pubkey_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
    let msg_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
    let msg_size = u16::from_le_bytes([data[12], data[13]]) as usize;

    // extract signature
    if sig_offset + 64 > data.len() {
        return None;
    }
    let mut signature = [0u8; 64];
    signature.copy_from_slice(&data[sig_offset..sig_offset + 64]);

    // extract pubkey
    if pubkey_offset + 32 > data.len() {
        return None;
    }
    let mut pubkey = [0u8; 32];
    pubkey.copy_from_slice(&data[pubkey_offset..pubkey_offset + 32]);

    // extract message
    if msg_offset + msg_size > data.len() {
        return None;
    }
    let message = data[msg_offset..msg_offset + msg_size].to_vec();

    Some(ParsedEd25519 {
        sig_count,
        pubkey,
        message,
    })
}
