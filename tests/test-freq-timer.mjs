// Test how CPU frequency scaling affects timer alarm timing
// The timer runs on the APB clock, which is derived from the chip's cycle counter.
// The cpuFrequency scaling only scales reported nanos, not actual cycles.
// So timer alarms should fire at the same wall time regardless of frequency setting.

import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

function findPeripheral(chip, baseAddr) {
  for (const p of chip.__peripherals || []) {
    if (p.baseAddr === baseAddr) return p;
  }
  // Try via CPU peripheral list
  const cpu = chip.cores?.[0];
  if (!cpu) return null;
  for (const p of cpu.peripherals || []) {
    if (p[1] === baseAddr || p.baseAddr === baseAddr) return p;
  }
  return null;
}

// Instead of complex peripheral lookup, let's use the simpler FrcTimer
// which is available at 0x3ff47000 on ESP32 and uses Timer32Counter

async function run(label, freq) {
  console.log(`\n--- ${label} (freq=${freq}) ---`);
  const chip = new ESP32({ wifi: false, cpuFrequency: freq });
  chip.loadROM(rom);
  chip.reset();

  // Compute nanoseconds multiplier
  const native = chip._nativeFrequency || 160e6;
  const freqHz = freq === 'max' ? native : (freq === 'auto' ? 8e6 : Number(freq) * 1e6);
  const multiplier = native / freqHz;
  console.log(`  native=${(native/1e6).toFixed(0)}MHz  target=${(freqHz/1e6).toFixed(0)}MHz  nanosMultiplier=${multiplier.toFixed(1)}x`);

  // Find TIMG0 by scanning known peripheral addresses
  // TIMG0 base = 0x3ff5f000 on ESP32
  let timg0 = null;
  const cpu = chip.cores[0];
  if (cpu && cpu.memory) {
    const map = cpu.memory.blocks || [];
    for (const block of map) {
      if (block.baseAddr === 0x3ff5f000 && block.data) {
        timg0 = block;
        break;
      }
    }
  }

  if (timg0) {
    console.log(`  TIMG0 found at 0x3ff5f000`);
    // Configure timer channel 0
    // Channel config at offset 0: enable=1, mode=0(inc), prescaler=1(2), alarm_en=1, clock_src=0(APB)
    const view = new DataView(timg0.data.buffer, timg0.data.byteOffset + timg0.data.byteOffset);
    timg0.data[0] = 0;  // bit 0: enable, bit 1: increase, bits 2-3: prescaler, bit 4: alarm enable
  } else {
    console.log(`  TIMG0 not found via memory blocks`);
  }

  // Step the chip for a fixed number of cycles
  const stepTarget = 16000000; // 16M cycles ≈ 100ms at 160MHz
  const t0 = Date.now();
  for (let i = 0; i < stepTarget; i++) {
    chip.step();
  }
  const dt = Date.now() - t0;
  const nanos = (chip.cycles / 160e6) * 1e9;
  console.log(`  cycles=${chip.cycles}  wall=${dt}ms  rawNanos=${nanos}`);
  console.log(`  scaledNanos=${nanos * multiplier}`);
}

console.log('=== Timer Test: Same cycle count, different frequency settings ===\n');
console.log('The chip always runs at native speed (160MHz). cpuFrequency only scales reported nanos.\n');

await run('160MHz', 'max');
await run('80MHz', 80);
await run('8MHz', 'auto');

console.log('\n=== Expected ===');
console.log('All three should show same wall time for the same cycle count.');
console.log('Only nanoseconds differs (scaled by multiplier).');
process.exit(0);
