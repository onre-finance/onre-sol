use super::*;

pub fn serialize_approval_message(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    expiry_unix: u64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(72);
    buf.extend_from_slice(program_id.as_ref());
    buf.extend_from_slice(user_pubkey.as_ref());
    buf.extend_from_slice(&expiry_unix.to_le_bytes());
    buf
}

pub fn build_ed25519_verify_ix(approver: &Keypair, message: &[u8]) -> Instruction {
    let signature = approver.sign_message(message);
    let sig_bytes = <[u8; 64]>::from(signature);
    let pubkey_bytes = approver.pubkey().to_bytes();

    let public_key_offset: u16 = 16;
    let signature_offset: u16 = 48;
    let message_data_offset: u16 = 112;
    let message_data_size: u16 = message.len() as u16;

    let mut data = Vec::with_capacity(112 + message.len());
    data.push(1u8);
    data.push(0u8);
    data.extend_from_slice(&signature_offset.to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&public_key_offset.to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&message_data_offset.to_le_bytes());
    data.extend_from_slice(&message_data_size.to_le_bytes());
    data.extend_from_slice(&u16::MAX.to_le_bytes());
    data.extend_from_slice(&pubkey_bytes);
    data.extend_from_slice(&sig_bytes);
    data.extend_from_slice(message);

    Instruction {
        program_id: ED25519_PROGRAM_ID,
        accounts: vec![],
        data,
    }
}
