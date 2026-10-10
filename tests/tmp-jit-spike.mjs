// JIT spike M1: minimal JS-side WASM emitter → compile → run, timed.
// Validates the toolchain path (emit/compile/instantiate/call overhead)
// and brackets the prize: pure-locals loop vs memory-backed loop vs the
// interpreter's ~130ns/guest-iter on the same shape.
function uleb(n) {
  const out = [];
  do { let b = n & 0x7f; n >>>= 7; if (n) b |= 0x80; out.push(b); } while (n);
  return out;
}
function sleb(n) {
  const out = [];
  let more = true;
  while (more) {
    let b = n & 0x7f;
    n >>= 7;
    if ((n === 0 && (b & 0x40) === 0) || (n === -1 && (b & 0x40) !== 0)) more = false;
    else b |= 0x80;
    out.push(b);
  }
  return out;
}
const OP = {
  end: 0x0b, block: 0x02, loop: 0x03, br: 0x0c, br_if: 0x0d,
  local_get: 0x20, local_set: 0x21, local_tee: 0x22,
  i32_const: 0x41, i32_add: 0x6a, i32_mul: 0x6c, i32_lt_u: 0x49,
  i32_load: 0x28, i32_store: 0x36, drop: 0x1a,
};
const I32 = 0x7f, VOID = 0x40;
function buildModule(body, nLocals, memPages, resultType) {
  const magic = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
  const typeSec = [0x01, ...uleb(6), 0x01, 0x60, 0x01, 0x7f, 0x01, resultType];
  const funcSec = [0x03, ...uleb(2), 0x01, 0x00];
  const memSec = [0x05, ...uleb(3), 0x01, 0x00, memPages];
  const expSec = [0x07, ...uleb(5), 0x01, 0x01, 0x66, 0x00, 0x00];
  const locDecl = nLocals ? [0x01, nLocals, 0x7f] : [0x00];
  const codeBody = [...locDecl, ...body, OP.end];
  const codeSec = [0x0a, ...uleb(codeBody.length + 2), 0x01, ...uleb(codeBody.length), ...codeBody];
  return new Uint8Array([...magic, ...typeSec, ...funcSec, ...memSec, ...expSec, ...codeSec]);
}
// f(N): acc=seed; i=0; do { acc=acc*K+C; i++ } while (i<N); return acc.
// locals: 0=N(param), 1=acc, 2=i
function traceBody() {
  const B = [];
  const c = (v) => B.push(OP.i32_const, ...sleb(v));
  const g = (i) => B.push(OP.local_get, ...uleb(i));
  const s = (i) => B.push(OP.local_set, ...uleb(i));
  c(0x12345678); s(1);
  c(0); s(2);
  B.push(OP.block, I32, OP.loop, VOID);
  g(1); c(1103515245); B.push(OP.i32_mul); c(12345); B.push(OP.i32_add); s(1);
  g(2); c(1); B.push(OP.i32_add); s(2);
  g(2); g(0); B.push(OP.i32_lt_u, OP.br_if, 0x00);
  B.push(OP.end);
  g(1);
  B.push(OP.end);
  return B;
}
async function main() {
  const mod = buildModule(traceBody(), 3, 1, I32);
  const tC0 = Date.now();
  const inst = await WebAssembly.instantiate(mod, {});
  const tC1 = Date.now();
  console.log(`[spike] module bytes=${mod.length} compile+instantiate=${tC1 - tC0}ms`);
  const N = Number(process.env.SPIKE_N || 10000000);
  const t0 = process.hrtime.bigint();
  const r = inst.instance.exports.f(N);
  const us = Number(process.hrtime.bigint() - t0) / 1000;
  let a = 0x12345678 >>> 0;
  for (let i = 0; i < N; i++) a = (Math.imul(a, 1103515245) + 12345) >>> 0;
  console.log(`[spike] emitted: result=0x${(r >>> 0).toString(16)} expect=0x${a.toString(16)} match=${(r >>> 0) === a}`);
  console.log(`[spike] ${N} iters in ${(us / 1e6).toFixed(2)}s = ${(N / us).toFixed(1)} Miter/s (~${(us / N).toFixed(1)}ns/iter)`);

  // Trace B: state in linear memory (the honest shape — regs live in the
  // shared buffer like v86; acc at byte 0). load+op+store per iter.
  const B2 = [];
  const c2 = (v) => B2.push(OP.i32_const, ...sleb(v));
  const g2 = (i) => B2.push(OP.local_get, ...uleb(i));
  const s2 = (i) => B2.push(OP.local_set, ...uleb(i));
  const ld = () => { B2.push(OP.i32_const, ...sleb(0), OP.i32_load, 0x02, 0x00); };
  const st = () => { B2.push(OP.local_tee, ...uleb(1), OP.i32_const, ...sleb(0), OP.local_get, ...uleb(1), OP.i32_store, 0x02, 0x00, OP.drop); };
  c2(0x12345678); st(); // mem[0]=seed (st balances its own stack)
  c2(0); s2(2);
  B2.push(OP.block, I32, OP.loop, VOID);
  ld(); c2(1103515245); B2.push(OP.i32_mul); c2(12345); B2.push(OP.i32_add); st();
  g2(2); c2(1); B2.push(OP.i32_add); s2(2);
  g2(2); g2(0); B2.push(OP.i32_lt_u, OP.br_if, 0x00);
  B2.push(OP.end);
  ld();
  B2.push(OP.end);
  const mod2 = buildModule(B2, 3, 1, I32);
  const inst2 = await WebAssembly.instantiate(mod2, {});
  const t2 = process.hrtime.bigint();
  const r2 = inst2.instance.exports.f(N);
  const us2 = Number(process.hrtime.bigint() - t2) / 1000;
  console.log(`[spike] mem-backed: result=0x${(r2 >>> 0).toString(16)} match=${(r2 >>> 0) === a}`);
  console.log(`[spike] ${N} iters in ${(us2 / 1e6).toFixed(2)}s = ${(N / us2).toFixed(1)} Miter/s (~${(us2 / N).toFixed(1)}ns/iter)`);
}
main().catch(e => { console.error(e); process.exit(1); });
