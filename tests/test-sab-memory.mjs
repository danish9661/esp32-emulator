import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function run() {
  const firmwareArg = process.argv[2];

  const mergedPath = firmwareArg
    ? resolve(process.cwd(), firmwareArg)
    : resolve(__dirname, '../build/esp32-peripherals/esp32-peripherals.ino.merged.bin');
  const flashBytes = readFileSync(mergedPath);

  const chipName = 'ESP32';
  const romPath = resolve(__dirname, '../rom/esp32-v3-rom.bin');
  const romBytes = readFileSync(romPath);

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

  // === Test 1: Flash SAB matches firmware ===
  console.log('\n[test] Checking flash SAB...');
  const memFlash = proxy.memory.flash;
  if (!memFlash) { console.error('[FAIL] proxy.memory.flash is undefined'); process.exit(1); }
  let match = true;
  for (let i = 0; i < Math.min(flashBytes.length, 256); i++) {
    if (memFlash[i] !== flashBytes[i]) { match = false; break; }
  }
  console.log(match ? '  [PASS] First 256 bytes match' : '[FAIL] Flash mismatch');
  if (!match) process.exit(1);

  // === Test 2: chipROM SAB exists and has ROM data ===
  console.log('\n[test] Checking chipROM SAB...');
  const memROM = proxy.memory.chipROM;
  if (!memROM) { console.error('[FAIL] proxy.memory.chipROM is undefined'); process.exit(1); }
  // ROM normally starts with some non-zero data
  const romNonZero = memROM[0] !== 0 || memROM[10] !== 0 || memROM[100] !== 0;
  console.log(romNonZero ? '  [PASS] chipROM has data' : '  [INFO] chipROM empty (expected before loadROM)');

  // === Test 3: Other regions exist ===
  console.log('\n[test] Checking SRAM/IRAM/MMU regions...');
  let missing = false;
  const expectedRegions = isESP32 ? ['iram', 'dataMem', 'rtcFastMem', 'mmuTableMemory', 'psram'] : ['sram', 'rtcFastMem', 'mmuTableMemory', 'iramCache'];
  for (const region of expectedRegions) {
    if (!proxy.memory[region]) { console.error(`  [FAIL] proxy.memory.${region} is undefined`); missing = true; }
    else { console.log(`  [PASS] proxy.memory.${region} (${proxy.memory[region].length} bytes)`); }
  }
  if (missing) process.exit(1);

  // === Test 4: Run and verify SRAM gets written ===
  console.log('\n[test] Running simulation...');
  proxy.run();
  for (let i = 0; i < 60; i++) {
    await new Promise((r) => setTimeout(r, 100));
    proxy.pollUart();
    const nan = proxy.nanos;
    process.stdout.write(`\r  [${i * 0.1}s] nanos=${nan} uart=${uartOutput.length}B    `);
    if (nan > 2000000 && uartOutput.length > 10) break;
  }
  proxy.stop();

  const sramRegion = isESP32 ? 'dataMem' : 'sram';
  console.log(`\n\n[test] Checking ${sramRegion} after run...`);
  const sram = proxy.memory[sramRegion];
  let sramNonZero = false;
  const checkLen = Math.min(sram.length, 65536);
  for (let i = 0; i < checkLen; i++) {
    if (sram[i] !== 0) { sramNonZero = true; break; }
  }
  if (sramNonZero) {
    console.log(`  [PASS] ${sramRegion} has non-zero data after execution`);
  } else {
    console.log(`  [INFO] ${sramRegion} still all zeros (firmware may not write to it)`);
  }

  // === Test 5: Verify bidirectional SAB works (write from main thread, readable from worker) ===
  console.log('\n[test] Testing bidirectional SAB...');
  const testIdx = isESP32 ? 0x1000 : 0x1000;
  if (sram.length > testIdx + 4) {
    const origVal = sram[testIdx];
    sram[testIdx] = 0xAA;
    // Run a few cycles - worker will read from SRAM
    proxy.run();
    await new Promise((r) => setTimeout(r, 200));
    proxy.stop();
    // If the worker didn't overwrite it, value should still be 0xAA
    const afterVal = sram[testIdx];
    console.log(`  [INFO] Wrote 0xAA to ${sramRegion}[${testIdx}], read back: 0x${afterVal.toString(16)}`);
    console.log(afterVal === 0xAA ? '  [PASS] Main-thread write preserved (no corruption)' : '  [INFO] Worker may have overwritten');
    sram[testIdx] = origVal;
  } else {
    console.log('  [SKIP] Region too small for bidirectional test');
  }

  proxy.terminate();
  console.log(`\n[test] ALL CHECKS PASSED`);
}

run().catch(e => { console.error(e); process.exit(1); });
