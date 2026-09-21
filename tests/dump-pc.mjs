import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flash = readFileSync(join(__dirname, 'fw-proxy32.bin'));
const APP = 0x10000;
const segs = [];
for (let off = APP + 24, i = 0; i < flash[APP + 1]; i++) {
  const load = flash.readUInt32LE(off);
  const len = flash.readUInt32LE(off + 4);
  segs.push({ load, len, data: off + 8 });
  off += 8 + len;
}
function codeByte(addr) {
  for (const s of segs) if (addr >= s.load && addr < s.load + s.len) return flash[s.data + (addr - s.load)];
  return 0xff;
}
const bytes = (pc) => [codeByte(pc), codeByte(pc + 1), codeByte(pc + 2), codeByte(pc + 3)];
const reg = (r) => `a${r}`;

let lastLen = 2;
function decode(pc) {
  const [b0, b1, b2, b3] = bytes(pc);
  const w = (b0 & 0xff) | ((b1 & 0xff) << 8) | ((b2 & 0xff) << 16) | ((b3 & 0xff) << 24);
  const op0 = b0 & 7, op1 = (b0 >> 3) & 0xF, op2 = (b1 >> 4) & 0xF;
  const t = (b1 >> 8) & 0xF, s = (b1 >> 12) & 0xF;
  lastLen = 3;
  if (op0 === 0) {
    const ops = ['BZ', 'BNEZ', 'BGEZ', 'BLTZ', 'BGE', 'BLT', 'BGEU', 'BLTU'];
    const imm16 = ((((w >>> 8) << 8) | (w & 0xff)) << 2) << 16 >> 16;
    return `${ops[op1]} ${reg(t)}, 0x${(pc + 4 + imm16).toString(16)}`;
  }
  if (op0 === 1) {
    if (op1 === 5) return `L32I  ${reg(t)}, ${reg(s)}, ${(w >>> 16) * 4}`;
    if (op1 === 6) return `S32I  ${reg(t)}, ${reg(s)}, ${(w >>> 16) * 4}`;
    if (op1 === 4) return `L16UI ${reg(t)}, ${reg(s)}, ${(w >>> 16) * 2}`;
    if (op1 === 8) return `L8UI  ${reg(t)}, ${reg(s)}, ${w >>> 16}`;
    if (op1 === 9) return `S8I   ${reg(t)}, ${reg(s)}, ${w >>> 16}`;
    if (op1 === 13) return `L16SI ${reg(t)}, ${reg(s)}, ${(w >>> 16) * 2}`;
    return `?op0=1 op1=${op1}`;
  }
  if (op0 === 2) {
    const imm = ((w & 0xff) << 24) >> 24;
    const target = (pc + 4 + (imm << 2)) & 0xffffffff;
    return `L32R  ${reg(t)}, 0x${target.toString(16)} (literal)`;
  }
  if (op0 === 3) return `L32E  ${reg(t)}, ${reg(s)}, ${(w >>> 16) * 4}`;
  if (op0 === 5) {
    if ((b0 >> 4) === 0 && op2 === 0 && (b1 & 0xf) === 0) return 'RET';
    if ((b0 >> 4) === 0 && op2 === 1 && (b1 & 0xf) === 0) return 'RETW';
    const m = (b0 >> 4) | ((b1 & 0xf) << 4) | (b2 << 8) | (b3 << 16);
    return `CALL0 0x${(pc + 4 + ((m << 2) << 1 >> 1) >>> 0).toString(16)}`;
  }
  if (op0 === 6) {
    const imm = (w >>> 6) << 2;
    return `J 0x${(pc + 4 + ((imm << 14) >> 14)).toString(16)}`;
  }
  if (op0 === 7) {
    if (op1 === 2) return `BNEZ  ${reg(s)}, 0x${(pc + 4 + (((((b1 & 0xf) << 8) | b2) << 2) << 21 >> 21)).toString(16)}`;
    if (op1 === 1) return `BEQZ  ${reg(s)}, 0x${(pc + 4 + (((((b1 & 0xf) << 8) | b2) << 2) << 21 >> 21)).toString(16)}`;
    if (op1 === 6) return `BNE   ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    if (op1 === 7) return `BEQ   ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    if (op1 === 8) return `BLT   ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    if (op1 === 9) return `BGE   ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    if (op1 === 10) return `BLTU  ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    if (op1 === 11) return `BGEU  ${reg(t)}, ${reg(s)}, 0x${(pc + 4 + ((w >>> 16) << 2)).toString(16)}`;
    return `?op0=7 op1=${op1}`;
  }
  if (op0 === 8) {
    if (op1 === 0) return 'EXCW';
    if (op1 === 2) return `EXTUI ${reg(t)}, ${reg(s)}, ${(b0 >> 4) & 0xf}, ${(b1 >> 4) & 0xf}`;
    return `?op0=8 op1=${op1}`;
  }
  if (op0 === 9) {
    const ops = ['AND', 'OR', 'XOR', 'ADD', 'SUB', 'ADDX2', 'SUBX2', 'ADDX4', 'SUBX4', 'ADDX8', 'SUBX8', 'MULL', 'MULU', 'MULUH', 'MULUL', 'MULSH'];
    return `${ops[op1] || '?'} ${reg(t)}, ${reg(s)}, ${reg(op2)}`;
  }
  if (op0 === 10) {
    const map = [0, 1, 2, 3, 4, 6, 8, 12, -1, -2, -3, -4, -6, -8, -12, 16];
    lastLen = 2;
    return `MOVI.N ${reg(t)}, ${map[(b1 >> 4) & 0xf]}`;
  }
  if (op0 === 11) {
    const imm8 = ((w >>> 8) & 0xff) << 24 >> 24;
    const ops = ['ADDI', 'ADDMI', 'SUBI'];
    return `${ops[op1] || '?'} ${reg(t)}, ${reg(s)}, ${imm8}`;
  }
  if (op0 === 12) {
    if (op2 === 0) return `SLLI  ${reg(t)}, ${reg(s)}, ${(b1 >> 4) & 0xf}`;
    if (op2 === 1) return `SRLI  ${reg(t)}, ${reg(s)}, ${(b1 >> 4) & 0xf}`;
    if (op2 === 2) return `SRAI  ${reg(t)}, ${reg(s)}, ${(b1 >> 4) & 0xf}`;
    return `?op0=12`;
  }
  if (op0 === 13) {
    if (op1 === 0) return 'BREAK';
    return `?op0=13 op1=${op1}`;
  }
  if (op0 === 14) return `MOVI  ${reg(t)}, ${((w >>> 8) & 0xffff) << 16 >> 16}`;
  if (op0 === 15) {
    if (op1 === 0) { lastLen = 3; return 'NOP'; }
    if (op1 === 3) return 'ILL';
    if (op1 === 5) return 'ISYNC';
    return `?op0=15 op1=${op1}`;
  }
  return `?? op0=${op0} op1=${op1} w=0x${w.toString(16)}`;
}

function seq(from, to, label) {
  console.log(`=== ${label} ===`);
  let pc = from;
  while (pc < to) {
    const d = decode(pc);
    const hex = bytes(pc).slice(0, lastLen).map((x) => x.toString(16).padStart(2, '0')).join(' ');
    console.log(`0x${pc.toString(16)} (${hex}): ${d}`);
    pc += lastLen;
  }
}
seq(0x40085c80, 0x40085d60, 'spin region 0x40085c80-0x40085d60');
seq(0x40088800, 0x40088890, 'core1 region 0x40088800-0x40088890');