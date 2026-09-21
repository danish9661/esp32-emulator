// Browser MD5 shim (node:crypto alias target). Public-domain-style compact
// MD5 over a Uint8Array, verified byte-identical to node crypto for
// short + long + empty inputs. Exposes node-like createHash('md5').
const S = [7,12,17,22, 7,12,17,22, 7,12,17,22, 7,12,17,22,
           5,9,14,20, 5,9,14,20, 5,9,14,20, 5,9,14,20,
           4,11,16,23, 4,11,16,23, 4,11,16,23, 4,11,16,23,
           6,10,15,21, 6,10,15,21, 6,10,15,21, 6,10,15,21];
const K = (() => { const t = []; for (let i = 0; i < 64; i++) t.push(Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296) >>> 0); return t; })();
function rotl(x, c) { return ((x << c) | (x >>> (32 - c))) >>> 0; }
export function md5bytes(data) {
  const len = data.length;
  const bitLen = len * 8;
  const withOne = len + 1;
  const padLen = ((56 - (withOne % 64)) + 64) % 64;
  const total = withOne + padLen + 8;
  const msg = new Uint8Array(total);
  msg.set(data, 0);
  msg[len] = 0x80;
  const dv = new DataView(msg.buffer);
  dv.setUint32(total - 8, bitLen >>> 0, true);
  dv.setUint32(total - 4, Math.floor(bitLen / 4294967296) >>> 0, true);
  let a0 = 0x67452301, b0 = 0xefcdab89, c0 = 0x98badcfe, d0 = 0x10325476;
  const M = new Uint32Array(16);
  for (let off = 0; off < total; off += 64) {
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
      B = (B + rotl(F, S[i])) >>> 0;
    }
    a0 = (a0 + A) >>> 0; b0 = (b0 + B) >>> 0; c0 = (c0 + C) >>> 0; d0 = (d0 + D) >>> 0;
  }
  const out = new Uint8Array(16);
  const odv = new DataView(out.buffer);
  odv.setUint32(0, a0, true); odv.setUint32(4, b0, true);
  odv.setUint32(8, c0, true); odv.setUint32(12, d0, true);
  return out;
}
export function createHash(_algo) {
  const chunks = [];
  return { update(b) { chunks.push(Uint8Array.from(b)); return this; }, digest() {
    let n = 0; for (const c of chunks) n += c.length;
    const cat = new Uint8Array(n); let o = 0;
    for (const c of chunks) { cat.set(c, o); o += c.length; }
    return md5bytes(cat);
  } };
}
