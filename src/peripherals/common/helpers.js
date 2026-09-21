function byteToHex(value) {
  return ((value >> 4) & 15).toString(16) + (15 & value).toString(16);
}
function uint32ToHex(value) {
  return (
    byteToHex(value >> 24) +
    byteToHex(value >> 16) +
    byteToHex(value >> 8) +
    byteToHex(value)
  );
}
function hexToBytes(hex) {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++)
    out[i] = parseInt(hex.substr(2 * i, 2), 16);
  return out;
}
function bytesToHex(bytes) {
  return Array.from(bytes).map(byteToHex).join("");
}
function randomUint32() {
  return Math.floor(0x100000000 * Math.random());
}
function createFieldDescriptor(reg, shift, width) {
  return { reg, shift, mask: (1 << width) - 1 };
}

export {
  byteToHex,
  uint32ToHex,
  hexToBytes,
  bytesToHex,
  randomUint32,
  createFieldDescriptor,
};
