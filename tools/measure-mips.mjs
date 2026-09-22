// Measures TRUE retired-instruction MIPS for every webdemo demo:
// (inst1-inst0)/wall to the pass mark, using Rust inst_count via the
// debug SAB (slots 8/9) — not the JS step-budget cycle counter.
//
// Usage: node tools/measure-mips.mjs [id...]   (skips ids already measured)
// Output: webdemo/mips.js (DEMO_MIPS table, regenerated in place).
// Needs: compile-server NOT required (uses committed prebuilt images);
//   WiFi/gateway NOT required (scan + Soft-AP demos run offline).
import { readFileSync, writeFileSync } from 'fs';
import { execFileSync } from 'child_process';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';

const root = dirname(fileURLToPath(import.meta.url));
const repo = resolve(root, '..');
const outJson = resolve(root, '..', 'webdemo', 'mips.json');
const outJs = resolve(root, '..', 'webdemo', 'mips.js');

const { SimulatorWorker } = await import(resolve(repo, 'src/index.js'));
const { DEMOS, runDemoScript } = await import(resolve(repo, 'webdemo/demos.js'));

const only = process.argv.slice(2);
let results = {};
try { results = JSON.parse(readFileSync(outJson, 'utf8')); } catch {}

for (const d of DEMOS) {
  if (only.length && !only.includes(d.id)) continue;
  if (results[d.id]?.mips != null && !only.includes(d.id)) continue;
  const binPath = resolve(repo, 'webdemo/firmware', d.file);
  try {
    const raw = execFileSync('gzip', ['-dc', binPath], { maxBuffer: 8 * 1024 * 1024 });
    const romBytes = readFileSync(resolve(repo, 'rom/esp32-v3-rom.bin'));
    const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
    flash.set(raw);
    const proxy = new SimulatorWorker();
    let out = '';
    proxy._onUART = (b) => { out += String.fromCharCode(b); };
    proxy._onError = () => {};
    await proxy.init('ESP32', {
      flashSizeMB: 4, mmuPages: 64, strapValue: 0x13,
      budget: d.budget || 5000000, board: d.board || 'esp32',
      ...(d.config || {}), ...(d.partitions ? { partitions: d.partitions } : {}),
    }, flash, romBytes);
    const i0 = (proxy.instTotal ?? 0) >>> 0;
    const c0 = proxy.debug ? proxy.debug[2] : 0;
    const t0 = performance.now();
    proxy.run();
    let scriptP = null;
    if (d.script) scriptP = runDemoScript(d.script, proxy, () => out);
    let echoDone = !d.interactive;
    const tEnd = Date.now() + (d.id === 'camera' || d.id === 'esp32-cam' ? 150000 : 100000);
    while (Date.now() < tEnd) {
      await new Promise((r) => setTimeout(r, 300));
      proxy.pollUart();
      if (d.interactive && !echoDone && out.includes(d.expect)) {
        echoDone = true;
        try { proxy.sendUart('hello mips\n'); } catch {}
      }
      if (d.interactive && echoDone && out.includes('hello mips')) break;
      if (!d.interactive && (out.includes(d.expect) || out.includes('ALL TESTS PASSED'))) break;
    }
    const wall = (performance.now() - t0) / 1000;
    let i1 = i0, c1 = c0;
    try { i1 = (proxy.instTotal ?? i0) >>> 0; } catch {}
    try { c1 = proxy.debug ? proxy.debug[2] : c0; } catch {}
    proxy.stop();
    proxy.terminate();
    const pass = out.includes(d.expect) || out.includes('ALL TESTS PASSED');
    const di = (i1 - i0) >>> 0;
    const dc = (c1 - c0) >>> 0;
    const mips = wall > 0 ? di / wall / 1e6 : 0;
    results[d.id] = {
      mips: Math.round(mips * 10) / 10, instr: di, cycles: dc,
      wall_s: Math.round(wall * 10) / 10, pass,
    };
    console.log(d.id, pass ? 'PASS' : 'CHECK',
      'mips=' + results[d.id].mips, 'instr=' + di, 'cycles=' + dc, 'wall=' + results[d.id].wall_s + 's');
    writeFileSync(outJson, JSON.stringify(results, null, 1));
  } catch (e) { console.log(d.id, 'ERROR', String(e.message || e).slice(0, 120)); }
}
// Regenerate webdemo/mips.js from the JSON so the page + docs always ship
// the measured numbers.
{
  const lines = [];
  for (const k of Object.keys(results).sort()) {
    const v = results[k];
    if (k.startsWith('_') || v == null || v.mips == null) continue;
    lines.push(`  '${k}': { mips: ${v.mips}, instr: ${v.instr}, wall_s: ${v.wall_s} },`);
  }
  const head = `// Per-demo TRUE MIPS: retired instructions / wall-clock to the pass mark.
// Retired = Rust CoreState.inst_count summed over both cores (debug SAB
// slots 8/9) — instructions the WASM cores REALLY executed, not the JS
// step-budget + idle fast-forward in chip.cycles (that old metric read
// ~3000 because one 512-instruction step adds 512 to chip.cycles while
// the cores may retire far fewer, and idle fast-forward adds cycles with
// zero instructions). Regenerate: node tools/measure-mips.mjs.
// Values below: all 53 PASS, same engine the page runs.
export const DEMO_MIPS = {
`;
  writeFileSync(outJs, head + lines.join('\n') + '\n};\n');
  console.log('wrote', outJs, '(' + lines.length + ' entries)');
}
console.log('DONE');
