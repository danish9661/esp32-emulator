import { build } from 'esbuild';
import { mkdirSync, copyFileSync } from 'fs';
import { dirname } from 'path';

// Bundle the public API, the worker-proxy, and the Web Worker entry into
// dist/ (preserving the src/ directory layout via outbase, so relative binary
// paths keep resolving).
//
// worker-proxy is kept as its OWN self-contained file at dist/sab/worker-proxy.js
// (marked external so the main bundle doesn't inline it, and no splitting so the
// worker-spawn code stays inside it). Its `import.meta.url` then correctly
// resolves ./worker-entry.js -> dist/sab/worker-entry.js and ../engine|../rom.
await build({
  entryPoints: ['src/index.js', 'src/sab/worker-proxy.js', 'src/sab/worker-entry.js'],
  bundle: true,
  format: 'esm',
  platform: 'neutral',
  packages: 'external', // leave node builtins (fs/path/url/worker_threads) external
  external: ['./sab/worker-proxy.js'],
  outbase: 'src',
  outdir: 'dist',
  sourcemap: true,
  logLevel: 'info',
});

// Copy the engine + boot-ROM binaries into dist/ at the same relative paths
// the loader reads them from (../engine/... and ../rom/...).
const binaries = [
  ['src/engine/esp-xtensa/esp_engine_wasm.wasm', 'dist/engine/esp-xtensa/esp_engine_wasm.wasm'],
  ['src/rom/esp32-v3-rom.bin', 'dist/rom/esp32-v3-rom.bin'],
  ['src/index.d.ts', 'dist/index.d.ts'],
];
for (const [src, dest] of binaries) {
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(src, dest);
}
console.log('dist build complete');
