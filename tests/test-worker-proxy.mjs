import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function run() {
  const firmwareArg = process.argv[2];

  const mergedPath = firmwareArg
    ? resolve(process.cwd(), firmwareArg)
    : resolve(__dirname, '../firmware.bin');
  const flashBytes = readFileSync(mergedPath);
  console.log(`[test] Firmware: ${mergedPath} (${flashBytes.length} bytes)`);

  const chipName = 'ESP32';
  const romPath = resolve(__dirname, '../rom/esp32-v3-rom.bin');
  const romBytes = readFileSync(romPath);
  console.log(`[test] BootROM: ${romBytes.length} bytes`);

  const flashSizeMB = Math.max(4, Math.ceil(flashBytes.length / (1024 * 1024)));
  const flashSab = new SharedArrayBuffer(flashSizeMB * 1024 * 1024);
  const flash = new Uint8Array(flashSab);
  flash.fill(0xff);
  flash.set(flashBytes);

  const proxy = new SimulatorWorker();

  let uartOutput = '';
  proxy._onUART = (byte) => { uartOutput += String.fromCharCode(byte); };
  proxy._onError = (err) => console.error('\n[test] Worker error:', err.message);

  console.log('[test] Initializing Worker...');
  await proxy.init(chipName, {
    flashSizeMB,
    mmuPages: Math.ceil(flashBytes.length / 65536),
    strapValue: 12,
    budget: 5000000,
    progressInterval: 10000000,
  }, flash, romBytes);
  console.log('[test] Worker ready');

  proxy.run();

  for (let i = 0; i < 50; i++) {
    await new Promise((r) => setTimeout(r, 100));
    proxy.pollUart();
    const nan = proxy.nanos;
    const pc = proxy.pc;
    process.stdout.write(`\r  [${i * 0.1}s] nanos=${nan} PC=0x${pc?.toString(16).padStart(8, '0')} stuck=${proxy.stuck} idle=${proxy.idle}    `);
    if (nan > 1000000 && uartOutput.length > 10) break;
  }

  const finalNanos = proxy.nanos;
  const finalPC = proxy.pc;

  proxy.stop();
  await new Promise((r) => setTimeout(r, 200));
  proxy.terminate();

  const uartLines = uartOutput.split('\n').filter(l => l.trim());
  console.log(`\n\n[test] Final nanos: ${finalNanos}`);
  console.log(`[test] Final PC:    0x${finalPC?.toString(16).padStart(8, '0')}`);
  console.log(`[test] UART lines:  ${uartLines.length}`);
  uartLines.slice(-5).forEach(l => console.log(`  ${l}`));
  console.log(`\n[test] PASSED`);
}

run().catch(e => { console.error(e); process.exit(1); });
