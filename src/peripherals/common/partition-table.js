const PARTITION_MAGIC = 0x50AA;
const PARTITION_ENTRY_SIZE = 32;

// Minimal MD5 (checksum use only): avoids node:crypto so this module loads
// in browser bundles (WebCrypto has no MD5). Returns 16 bytes.
function md5sum(msg) {
  const s = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
  const K = [];
  for (let i = 0; i < 64; i++) K[i] = Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296) >>> 0;
  const l = msg.length;
  const bitLen = l * 8;
  const padded = new Uint8Array((((l + 8) >> 6) + 1) << 6);
  padded.set(msg);
  padded[l] = 0x80;
  const dv = new DataView(padded.buffer);
  dv.setUint32(padded.length - 8, bitLen >>> 0, true);
  dv.setUint32(padded.length - 4, Math.floor(bitLen / 4294967296), true);
  let a0 = 0x67452301, b0 = 0xefcdab89, c0 = 0x98badcfe, d0 = 0x10325476;
  const M = new Uint32Array(16);
  const rotl = (x, n) => (x << n) | (x >>> (32 - n));
  for (let off = 0; off < padded.length; off += 64) {
    for (let i = 0; i < 16; i++) M[i] = dv.getUint32(off + i * 4, true);
    let A = a0, B = b0, C = c0, D = d0;
    for (let i = 0; i < 64; i++) {
      let F, g;
      if (i < 16) { F = (B & C) | (~B & D); g = i; }
      else if (i < 32) { F = (D & B) | (~D & C); g = (5 * i + 1) % 16; }
      else if (i < 48) { F = B ^ C ^ D; g = (3 * i + 5) % 16; }
      else { F = C ^ (B | ~D); g = (7 * i) % 16; }
      F = (F + A + K[i] + M[g]) >>> 0;
      A = D; D = C; C = B;
      B = (B + rotl(F, s[i])) >>> 0;
    }
    a0 = (a0 + A) >>> 0; b0 = (b0 + B) >>> 0; c0 = (c0 + C) >>> 0; d0 = (d0 + D) >>> 0;
  }
  const out = new Uint8Array(16);
  const odv = new DataView(out.buffer);
  odv.setUint32(0, a0, true); odv.setUint32(4, b0, true);
  odv.setUint32(8, c0, true); odv.setUint32(12, d0, true);
  return out;
}
const PARTITION_TABLE_OFFSET = 0x8000;
const PARTITION_MAGIC_MD5 = 0xEBEB;

const TYPE_MAP = {
  app: 0x00,
  data: 0x01,
};

const SUBTYPE_MAP = {
  // app subtypes
  ota_0: 0x00,
  ota_1: 0x10,
  ota_2: 0x20,
  ota_3: 0x30,
  ota_4: 0x40,
  ota_5: 0x50,
  ota_6: 0x60,
  ota_7: 0x70,
  ota_8: 0x80,
  ota_9: 0x90,
  ota_10: 0xa0,
  ota_11: 0xb0,
  ota_12: 0xc0,
  ota_13: 0xd0,
  ota_14: 0xe0,
  ota_15: 0xf0,
  test: 0x00,
  // data subtypes
  ota: 0x00,
  phy: 0x01,
  nvs: 0x02,
  coredump: 0x03,
  nvs_keys: 0x04,
  efuse: 0x05,
  undefined: 0x06,
  esphttpd: 0x80,
  fat: 0x81,
  spiffs: 0x82,
};

function parseHex(v) {
  if (typeof v === "number") return v;
  return parseInt(v, 16);
}

function parseCSV(csvText) {
  const lines = csvText.split("\n");
  const entries = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const parts = line.split(",").map((s) => s.trim());
    if (parts.length < 5) continue;
    const [name, type, subtype, offset, size, flags] = parts;
    const typeVal = TYPE_MAP[type.toLowerCase()];
    if (typeVal === undefined) continue;
    let subtypeVal = SUBTYPE_MAP[subtype.toLowerCase()];
    if (subtypeVal === undefined) subtypeVal = parseHex(subtype);
    entries.push({
      name: name,
      type: typeVal,
      subtype: subtypeVal,
      offset: parseHex(offset),
      size: parseHex(size),
      flags: flags ? parseHex(flags) : 0,
    });
  }
  return entries;
}

function generateBinary(entries) {
  const n = entries.length;
  const totalBytes = (n + 1) * PARTITION_ENTRY_SIZE;
  const buf = new Uint8Array(totalBytes);
  const view = new DataView(buf.buffer);
  const enc = new TextEncoder();
  // Partition entries (no header - first entry starts at offset 0)
  for (let i = 0; i < n; i++) {
    const e = entries[i];
    const off = i * PARTITION_ENTRY_SIZE;
    view.setUint16(off, PARTITION_MAGIC, true);
    view.setUint8(off + 2, e.type);
    view.setUint8(off + 3, e.subtype);
    view.setUint32(off + 4, e.offset, true);
    view.setUint32(off + 8, e.size, true);
    const label = enc.encode(e.name);
    for (let j = 0; j < 16; j++) {
      view.setUint8(off + 12 + j, j < label.length ? label[j] : 0);
    }
  }
  // End marker (part of the last entry slot)
  const endOff = n * PARTITION_ENTRY_SIZE;
  view.setUint16(endOff, PARTITION_MAGIC_MD5, true);
  for (let i = 2; i < 16; i++) view.setUint8(endOff + i, 0xFF);
  // Compute MD5 over partition entries only (per gen_esp32part.py spec).
  // Pure-JS MD5 (checksum use): node:crypto is unavailable in browser
  // bundles and WebCrypto has no MD5; keeps this module sync + portable.
  const md5 = md5sum(buf.subarray(0, endOff));
  buf.set(new Uint8Array(md5), endOff + 16);
  return buf;
}

function writePartitionTable(flash, csvText) {
  const entries = parseCSV(csvText);
  if (entries.length === 0) return;
  const binary = generateBinary(entries);
  const end = PARTITION_TABLE_OFFSET + binary.length;
  if (end > flash.length) {
    console.warn("Partition table exceeds flash size");
    return;
  }
  flash.set(binary, PARTITION_TABLE_OFFSET);
  // Clear remaining bytes in the 4K sector to prevent stale data from being
  // interpreted as additional partition entries or end markers
  const sectorEnd = PARTITION_TABLE_OFFSET + 0x1000;
  const clearStart = PARTITION_TABLE_OFFSET + binary.length;
  const clearEnd = Math.min(sectorEnd, flash.length);
  if (clearEnd > clearStart) {
    flash.fill(255, clearStart, clearEnd);
  }
}

function parseMacAddress(str) {
  if (!str || typeof str !== "string") return null;
  const parts = str.split(":").map((s) => parseInt(s, 16));
  if (parts.length !== 6 || parts.some((p) => isNaN(p) || p < 0 || p > 255)) return null;
  return parts;
}

function parseFirmwareOffset(v) {
  if (v === undefined || v === null || v === "") return 0;
  if (typeof v === "number") return v;
  if (typeof v === "string") return parseInt(v, 16);
  return 0;
}

export {
  parseCSV,
  generateBinary,
  writePartitionTable,
  parseMacAddress,
  parseFirmwareOffset,
  PARTITION_TABLE_OFFSET,
};
