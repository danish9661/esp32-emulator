#!/usr/bin/env node
// esp32emu CLI — boot ESP32 firmware and stream UART.
import { SimulatorWorker } from '../dist/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const HELP = `
esp32emu — ESP32 (Xtensa LX6, dual-core) WASM emulator CLI

Usage:
  esp32emu run <firmware.bin> [options]   Boot firmware and stream UART to stdout
  esp32emu help                           Show this help
  esp32emu version                        Print the package version

run options:
  --flash-size <MB>   Flash size in MB (default 4)
  --strap <hex>       GPIO strap value, e.g. 0x13 (default 0x13)
  --budget <n>        Instructions per run chunk (default 500000)
  --once              Exit shortly after boot instead of running until Ctrl-C
  -h, --help          Show this help

Examples:
  esp32emu run build/firmware.bin
  esp32emu run firmware.bin --flash-size 8 --once

Notes:
  The ESP32 boot ROM and the WASM engine are bundled with the package, so no
  extra files are required. <firmware.bin> is an ESP32 firmware image flashed
  at offset 0x1000 (e.g. the .bin produced by the Arduino/ESP-IDF build).
`;

function parseArgs(argv) {
  const opts = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '-h' || a === '--help') opts.help = true;
    else if (a === '--flash-size') opts.flashSize = Number(argv[++i]);
    else if (a === '--strap') opts.strap = parseInt(argv[++i], 16);
    else if (a === '--budget') opts.budget = Number(argv[++i]);
    else if (a === '--once') opts.once = true;
    else if (a.startsWith('--')) { console.error(`Unknown option: ${a}`); process.exit(2); }
    else opts._.push(a);
  }
  return opts;
}

const VERSION = JSON.parse(readFileSync(resolve(__dirname, '../package.json'), 'utf8')).version;

async function runFirmware(opts) {
  const positional = opts._;
  const file = positional[0] === 'run' ? positional[1] : positional[0];

  if (!file) {
    console.error('error: run requires a firmware .bin path\n');
    console.error(HELP);
    process.exit(2);
  }
  let bin;
  try {
    bin = readFileSync(file);
  } catch (e) {
    console.error(`error: cannot read firmware '${file}': ${e.message}`);
    process.exit(1);
  }

  const flashSizeMB = opts.flashSize || 4;
  const flash = new Uint8Array(flashSizeMB * 1024 * 1024).fill(0xff);
  if (bin.length > flash.length) {
    console.error(`error: firmware (${bin.length} bytes) does not fit in ${flashSizeMB}MB flash`);
    process.exit(1);
  }
  flash.set(bin, 0);

  const sim = new SimulatorWorker();
  sim._onUART = (b) => { process.stdout.write(String.fromCharCode(b)); };
  sim._onError = (e) => {
    console.error('\n[esp32emu] runtime error:', e && e.message ? e.message : e);
    process.exit(1);
  };

  try {
    await sim.init('ESP32', {
      flashSizeMB,
      mmuPages: 64,
      strapValue: opts.strap ?? 0x13,
      budget: opts.budget || 500000,
    }, flash);
  } catch (e) {
    console.error(`error: failed to initialize the simulator: ${e && e.message ? e.message : e}`);
    console.error('Ensure the esp32emu package files are intact (bundled boot ROM + WASM engine).');
    process.exit(1);
  }

  console.error(`[esp32emu] booting ${file} (${bin.length} bytes) — Ctrl-C to stop`);
  sim.run();
  const poll = setInterval(() => sim.pollUart(), 50);

  if (opts.once) {
    setTimeout(() => { clearInterval(poll); try { sim.terminate(); } catch {} process.exit(0); }, 1500);
    return;
  }

  const shutdown = () => {
    clearInterval(poll);
    try { sim.terminate(); } catch {}
    process.exit(0);
  };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
}

(async () => {
  const opts = parseArgs(process.argv.slice(2));
  const positional = opts._;
  const cmd = positional[0];

  if (opts.help || cmd === 'help') { console.log(HELP); process.exit(0); }
  if (cmd === 'version') { console.log(VERSION); process.exit(0); }
  if (cmd === 'run' || (cmd && cmd.endsWith('.bin'))) { await runFirmware(opts); return; }
  if (!cmd) { console.log(HELP); process.exit(0); }
  console.error(`Unknown command: ${cmd}\n`);
  console.error(HELP);
  process.exit(2);
})();
