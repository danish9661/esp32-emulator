// Build tests/browser-bench/www from current source (bundles + binaries).
import { execFileSync } from 'child_process';
import { copyFileSync, mkdirSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..', '..');
const out = join(root, 'tests', 'browser-bench', 'www');
mkdirSync(out, { recursive: true });
const esbuild = join(root, 'node_modules', '.bin', 'esbuild');
const ext = ['--external:worker_threads', '--external:node:crypto', '--external:crypto', '--external:node:net', '--external:node:os'];
const run = (entry, outfile) =>
  execFileSync(esbuild, [entry, '--bundle', '--format=esm', '--platform=browser', ...ext, `--outfile=${outfile}`], { cwd: root, stdio: 'inherit' });
run('src/sab/worker-entry.js', join(out, 'worker-entry.js'));
run('src/sab/worker-proxy.js', join(out, 'worker-proxy.js'));
copyFileSync(join(root, 'src/engine/esp-xtensa/esp_engine_wasm.wasm'), join(out, 'esp_engine_wasm.wasm'));
copyFileSync(join(root, 'tests/rom/esp32-v3-rom.bin'), join(out, 'esp32-v3-rom.bin'));
copyFileSync(join(root, 'webdemo/firmware/fw-gpio.bin.gz'), join(out, 'fw-gpio.bin.gz'));
copyFileSync(join(root, 'tests/browser-bench/bench.html'), join(out, 'bench.html'));
console.log('bench www ready:', out);
