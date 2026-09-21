// Shared helper functions extracted from original factory closure

export function gcmMultiply(block, key) {
  const result = new Uint32Array(4);
  const keyArr = new Uint32Array(key);
  for (let i = 0; i < 128; i++) {
    if ((block[i >> 5] & (1 << (31 - (i % 32)))) !== 0) {
      result[0] ^= keyArr[0];
      result[1] ^= keyArr[1];
      result[2] ^= keyArr[2];
      result[3] ^= keyArr[3];
    }
    const carry = keyArr[3] & 1 ? 0xe1000000 : 0;
    keyArr[3] = (keyArr[3] >>> 1) | ((keyArr[2] & 1) << 31);
    keyArr[2] = (keyArr[2] >>> 1) | ((keyArr[1] & 1) << 31);
    keyArr[1] = (keyArr[1] >>> 1) | ((keyArr[0] & 1) << 31);
    keyArr[0] = (keyArr[0] >>> 1) ^ carry;
  }
  block.set(result);
}

export function findFirstSetBit(value) {
  for (let t = 0; t < 32; t++) if (value & (1 << t)) return t;
  return 32;
}

export function byteSwap32(value) {
  return (
    ((255 & value) << 24) |
    ((65280 & value) << 8) |
    ((value >> 8) & 65280) |
    ((value >> 24) & 255)
  );
}

export function maskLowBits(value, bits) {
  return bits >= 32 ? value : value & ((1 << bits) - 1);
}

export function signExtend(value, bits) {
  const shift = 32 - bits;
  return (value << shift) >> shift;
}

export function clampInt8(value) {
  return value > 127 ? 127 : value < -128 ? -128 : value;
}

export function clampInt16(value) {
  return value > 32767 ? 32767 : value < -32768 ? -32768 : value;
}

export function clampInt32(value) {
  return value > 0x7fffffff
    ? 0x7fffffff
    : value < -0x80000000
      ? -0x80000000
      : value;
}
