pub struct ParsedEd25519 {
    pub sig_count: u8,
    pub pubkey: [u8; 32],
    pub message: Vec<u8>,
}

/// Parse Ed25519 verify instruction data into useful parts.
/// Returns None if data is malformed.
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