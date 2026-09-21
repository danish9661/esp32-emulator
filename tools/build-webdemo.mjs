// Rebuilds the browser bundles used by webdemo/worker/ from src/.
//
// What it does:
//   1. Bundles src/sab/worker-entry.js (+ the whole engine it imports) for
//      the browser, aliasing node:crypto to tools/md5-shim.mjs and leaving
//      worker_threads/fs/path/url external (dead branches in the browser;
//      the worker posts via self.onmessage there).
//   2. Bundles src/sab/worker-proxy.js the same way (the dynamic
//      worker_threads import is browser-dead code, kept external).
//   3. Freshens webdemo/worker/esp_engine_wasm.wasm + esp32-v3-rom.bin from
//      src/ so the deployed site always matches the current engine.
//
// Run:  node tools/build-webdemo.mjs
// Needs: npm install (esbuild). No Arduino toolchain, no compile server —
//   webdemo/firmware/*.bin.gz are prebuilt and committed, rebuilt only by
//   re-running the peripheral suite locally.

import { build } from 'esbuild';
import { copyFileSync } from 'fs';

await build({
  entryPoints: ['src/sab/worker-entry.js'],
  bundle: true,
  format: 'esm',
  platform: 'browser',
  outfile: 'webdemo/worker/worker-entry.js',
  logLevel: 'info',
  external: ['worker_threads', 'fs', 'path', 'url'],
  alias: { 'node:crypto': './tools/md5-shim.mjs' },
});

await build({
  entryPoints: ['src/sab/worker-proxy.js'],
  bundle: true,
  format: 'esm',
  platform: 'browser',
  outfile: 'webdemo/worker/worker-proxy.js',
  logLevel: 'info',
  external: ['worker_threads'],
});

// Silence the firmware-driven peripheral-reset log: on real hardware those
// reset lines touch real registers; here the Rust side owns them, so the JS
// lookup misses and console.error spams the devtools console every boot.
// (Kept as a no-op rather than deleting the call, so Node diagnostics stay.)
{
  const path = 'webdemo/worker/worker-entry.js';
  const { readFileSync, writeFileSync } = await import('fs');
  let s = readFileSync(path, 'utf8');
  const needle =
    'regIdx ? (applySingleResetValues(regIdx, Esp32FullResetValues), regIdx.reset()) : console.error("Peripheral to reset not found", addr.toString(16));';
  if (!s.includes(needle)) {
    console.warn('[build-webdemo] reset-spam patch: needle not found, skipping');
  } else {
    s = s.replace(
      needle,
      'if (regIdx) { applySingleResetValues(regIdx, Esp32FullResetValues); regIdx.reset(); }',
    );
    writeFileSync(path, s);
    console.log('[build-webdemo] reset-spam patch applied');
  }
}

// Browser-safe UART TX path: the Node proxy uses Buffer.from, which does not
// exist in browsers. Fall back to TextEncoder so page-driven sendUart works
// (UART echo demo + pcnt/mcpwm/i2c-slave host keypress scripts).
{
  const path = 'webdemo/worker/worker-proxy.js';
  const { readFileSync, writeFileSync } = await import('fs');
  let s = readFileSync(path, 'utf8');
  const needle =
    'const bytes = typeof data === "string" ? Buffer.from(data, "utf-8") : data;';
  if (!s.includes(needle)) {
    console.warn('[build-webdemo] sendUart patch: needle not found, skipping');
  } else {
    s = s.replace(
      needle,
      'const bytes = typeof data === "string" ? (typeof Buffer !== "undefined" ? Buffer.from(data, "utf-8") : new TextEncoder().encode(data)) : data;',
    );
    writeFileSync(path, s);
    console.log('[build-webdemo] sendUart browser patch applied');
  }
}

copyFileSync(
  'src/engine/esp-xtensa/esp_engine_wasm.wasm',
  'webdemo/worker/esp_engine_wasm.wasm',
);
copyFileSync('src/rom/esp32-v3-rom.bin', 'webdemo/worker/esp32-v3-rom.bin');
console.log('[build-webdemo] worker binaries freshened');
