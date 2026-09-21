use super::sha::ShaEngine;

pub fn sha256_hash(input: &[u8]) -> [u8; 32] {
    let mut engine = ShaEngine::new();
    engine.initialize(2);

    let len = input.len();
    let bit_len = (len as u64) * 8;

    let mut pos = 0;
    while pos + 64 <= len {
        engine.update(2, &input[pos..pos + 64]);
        pos += 64;
    }

    let remaining = len - pos;
    let mut block = [0u8; 64];
    block[..remaining].copy_from_slice(&input[pos..]);
    block[remaining] = 0x80;

    if remaining < 56 {
        block[56..64].copy_from_slice(&bit_len.to_be_bytes());
        engine.update(2, &block);
    } else {
        engine.update(2, &block);
        let mut pad_block = [0u8; 64];
        pad_block[56..64].copy_from_slice(&bit_len.to_be_bytes());
        engine.update(2, &pad_block);
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        let bytes = engine.hash[i].to_be_bytes();
        out[4 * i..4 * i + 4].copy_from_slice(&bytes);
    }
    out
}

pub fn sha256_raw(input: &[u8]) -> [u8; 32] {
    let mut engine = ShaEngine::new();
    engine.initialize(2);

    let mut pos = 0;
    while pos + 64 <= input.len() {
        engine.update(2, &input[pos..pos + 64]);
        pos += 64;
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        let bytes = engine.hash[i].to_be_bytes();
        out[4 * i..4 * i + 4].copy_from_slice(&bytes);
    }
    out
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() <= 64 {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key_block[i] ^ 0x36;
        opad[i] = key_block[i] ^ 0x5c;
    }

    let mut inner = ShaEngine::new();
    inner.initialize(2);
    inner.update(2, &ipad);

    let mut pos = 0;
    while pos + 64 <= data.len() {
        inner.update(2, &data[pos..pos + 64]);
        pos += 64;
    }

    let mut inner_hash = [0u8; 32];
    for i in 0..8 {
        let bytes = inner.hash[i].to_be_bytes();
        inner_hash[4 * i..4 * i + 4].copy_from_slice(&bytes);
    }

    let mut combined = [0u8; 96];
    combined[..64].copy_from_slice(&opad);
    combined[64..].copy_from_slice(&inner_hash);
    sha256_hash(&combined)
}
