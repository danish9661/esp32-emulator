// WASM stack-height simulator for emitter output: catches imbalance
// WITHOUT instantiating (instant, no engine boot). Supports the opcodes
// the emitter uses; anything else throws.
import { buildTraceModule } from './tmp-jit-lib.mjs';
import { readFileSync } from 'fs';
const ops = process.argv[2] === 'ALL'
  ? [...new Set(readFileSync('tests/tmp-emit-micro.mjs', 'utf8').match(/op: (0x[0-9a-f]+)/g).map((m) => parseInt(m.slice(4))))]
  : process.argv[2].split(',').map((x) => parseInt(x));
process.envConsts = process.env.PRINTconsts ? [] : null;
const TRACE = process.env.TRACE === '1';
const T = ops.map((op, i) => ({ pc: 0x3ffe8000 + i * 3, op, w: 3, next: 0 }));
T.forEach((e, i) => { e.next = (i + 1 < T.length) ? T[i + 1].pc : (e.pc + e.w); });
const built = buildTraceModule(T, { CORE: 0, CBASE: parseInt(process.env.CBASE || '0', 10), wb: 0, recLoop: [0,0,0], sarOff: 100, readMem: () => 0 });
const b = [...built.mod];
// find code section function body (last section 0x0a, single func, skip decl)
let i = 8; // skip magic+version
const pop = () => b[i++];
const readU = () => { let r = 0, s = 0, x; do { x = pop(); r |= (x & 0x7f) << s; s += 7; } while (x & 0x80); return r >>> 0; };
let body = null;
while (i < b.length) {
  const sec = pop(), sz = readU(), end = i + sz;
  if (sec === 10) {
    const n = readU();
    const fsz = readU();
    const fstart = i;
    const nloc = readU();
    for (let k = 0; k < nloc; k++) { readU(); pop(); }
    body = { start: fstart, code: b.slice(i, fstart + fsz) };
  }
  i = end;
}
if (!body) throw new Error('no body');
// simulate
const st = []; // stack of heights is enough (track depth only)
let p = 0;
const code = body.code;
const depth = [0];
const err = (m) => { throw new Error(`@${p}: ${m} (depth=${JSON.stringify(depth)}) ctx=${code.slice(Math.max(0,p-12),p+4).map((x)=>x.toString(16).padStart(2,'0')).join(' ')}`); };
const push = (n) => { depth[depth.length - 1] += n; };
const rSleb = () => { let r = 0, s = 0, x; do { x = code[p++]; r |= (x & 0x7f) << s; s += 7; } while (x & 0x80); };
const blockStack = [];
let unreachable = false;
while (p < code.length) {
  const o = code[p++];
  if (TRACE) console.log(`  @${p - 1} op=0x${o.toString(16)} depth=${JSON.stringify(depth)}${unreachable ? ' (unreach)' : ''}`);
  const top = () => depth[depth.length - 1];
  const need = (n) => { if (top() < n) err(`need ${n} have ${top()} op=0x${o.toString(16)}`); };
  if (unreachable && o !== 0x05 && o !== 0x0b) {
    // skip height updates but still consume immediates
    if (o === 0x41 || o === 0x42) rSleb();
    else if (o === 0x28 || o === 0x36 || o === 0x2a || o === 0x38) { rSleb(); rSleb(); }
    else if (o === 0x20 || o === 0x21 || o === 0x22 || o === 0x10) p++;
    else if (o === 0x04) { pop(); blockStack.push({ end: depth[depth.length - 1], elseSeen: false }); }
    continue;
  }
  switch (o) {
    case 0x41: { let v = 0, s3 = 0, x; do { x = code[p++]; v |= (x & 0x7f) << s3; s3 += 7; } while (x & 0x80); if (v & (1 << (s3 - 1))) v -= (1 << s3); if (process.envConsts) process.envConsts.push(v); push(1); break; } // const
    case 0x28: rSleb(); rSleb(); need(1); break; // load: net 0
    case 0x36: rSleb(); rSleb(); need(2); push(-2); break; // store
    case 0x2a: rSleb(); rSleb(); need(1); break; // f32.load
    case 0x38: rSleb(); rSleb(); need(2); push(-2); break; // f32.store
    case 0x45: need(1); break; // eqz net 0
    case 0x46: case 0x47: case 0x48: case 0x49: need(2); push(-1); break; // cmp
    case 0x51: case 0x56: need(2); push(-1); break; // i64 cmp
    case 0x71: case 0x72: case 0x73: case 0x6a: case 0x6b: case 0x6c: case 0x6f:
    case 0x74: case 0x75: case 0x76: case 0x7c: case 0x86: case 0x88:
    case 0x92: case 0x94: need(2); push(-1); break; // f32 binops
    case 0x67: need(1); break; // clz
    case 0xa7: case 0xad: case 0xc1: need(1); break; // conversions
    case 0x42: rSleb(); push(1); break; // i64.const (slebs may be multi-byte; ok)
    case 0xbc: need(1); break; // i32.reinterpret_f32 (net 0)
    case 0xbe: need(1); break; // f32.reinterpret_i32 (net 0)
    case 0x43: p += 4; push(1); break; // f32.const
    case 0x8a: need(1); break; // f32.trunc
    case 0xfc: { const s = code[p++]; if (s === 2) { need(1); } else err('bad 0xfc subop'); break; }
    case 0x20: case 0x22: p++; push(o === 0x20 ? 1 : 0); break; // get/tee
    case 0x21: p++; need(1); push(-1); break; // set
    case 0x10: { const f = code[p++]; const nt = f <= 2 ? -1 : (f <= 5 ? -3 : -2); need(-nt + (f <= 5 ? (f <= 2 ? 1 : 0) : (f === 6 ? 1 : 0))); push(nt); break; } // call by import idx
    case 0x04: need(1); push(-1); blockStack.push({ end: top(), elseSeen: false }); break; // if
    case 0x05: { const fr = blockStack[blockStack.length - 1]; if (unreachable) { depth[depth.length - 1] = fr.end; unreachable = false; } else if (top() !== fr.end) err(`if/else height ${top()} != ${fr.end}`); fr.elseSeen = true; break; }
    case 0x0b: { const fr = blockStack.pop(); if (unreachable) { depth[depth.length - 1] = fr ? fr.end : 1; unreachable = false; } else if (fr && top() !== fr.end) err(`block end height ${top()} != ${fr.end}`); break; }
    case 0x1a: need(1); push(-1); break; // drop
    case 0x0f: unreachable = true; break; // return
    case 0x40: case 0x7f: case 0x7e: case 0x7d: case 0x7c: break; // block-type immediates
    default: err(`unknown opcode 0x${o.toString(16)}`);
  }
}
console.log('STACKSIM-OK final depth=' + depth);
if (process.envConsts) console.log('consts: ' + process.envConsts.join(' '));
