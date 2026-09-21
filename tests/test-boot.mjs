import { ESP32 } from '../src/index.js';
import { readFileSync } from 'fs';

const BOOTROM_PATH = './rom/esp32-v3-rom.bin';
const rom = readFileSync(BOOTROM_PATH);
console.log(`BootROM size: ${rom.length} bytes`);

// Flash: 4MB, fill with 0xFF
const flashSizeMB = 4;
const flash = new Uint8Array(flashSizeMB * 1024 * 1024);
flash.fill(0xff);

// Write a minimal valid ESP32 image header at 0x1000.
// ESP32 image format (offset from 0x1000):
//   [0]  = 0xe9 (magic)
//   [1]  = number of segments
//   [2]  = flash_mode (0x20=QIO)
//   [3]  = flash_freq (high nibble: size, low nibble: freq) 0x40 = 80MHz/4MB
//   [4-7]= entry_addr
//   [8]  = reserved
//   [12] = chip_id (0x0000=ESP32)
//   [16] = min chip rev
//   [17] = reserved
//   [18] = hash appended (bit 0)
// We'll create a tiny valid image: one segment that does nothing.
flash[0x1000] = 0xe9;          // magic
flash[0x1001] = 1;             // 1 segment
flash[0x1002] = 0x20;          // QIO mode
flash[0x1003] = 0x40;          // 80MHz / 4MB

// Entry point (where to jump after loading) — point to a safe ROM return
flash[0x1004] = 0x00;          // entry addr LSB
flash[0x1005] = 0x00;
flash[0x1006] = 0x00;
flash[0x1007] = 0x40;          // 0x40000000 -> reset vector (causes soft reset)

// Chip ID: ESP32 = 0x0000
flash[0x100c] = 0x00;
flash[0x100d] = 0x00;
flash[0x100e] = 0x00;
flash[0x100f] = 0x00;

// Segment header starts at offset 0x14 (after 20 bytes of image header):
//   [0-3] = load_addr
//   [4-7] = data_len
//   [8..] = data
flash[0x1014] = 0x00; flash[0x1015] = 0x00; flash[0x1016] = 0x00; flash[0x1017] = 0x40; // load at 0x40000000
flash[0x1018] = 0x04; flash[0x1019] = 0x00; flash[0x101a] = 0x00; flash[0x101b] = 0x00; // 4 bytes of data
flash[0x101c] = 0x00; // NOP-like padding
flash[0x101d] = 0x00;
flash[0x101e] = 0x00;
flash[0x101f] = 0x00;

// Checksum byte for the segment data [at end]
const checksum = 0xef; // placeholder
const chkOff = 0x1020;
flash[chkOff] = checksum;

const esp32 = new ESP32({ flashSizeMB, flash });
esp32.loadROM(rom);

console.log(`Cores: ${esp32.cores?.length}, Freq: ${esp32.clock?.frequency} Hz`);

// ---- Critical pre-init (from esp32-runner.ts) ----
if (esp32.gpio?.pins) {
    if (esp32.gpio.pins[0])  esp32.gpio.pins[0].inputValue = true;
    if (esp32.gpio.pins[2])  esp32.gpio.pins[2].inputValue = false;
    if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
    if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
}

esp32.reset();
esp32.flash.set(flash);

if (esp32.gpio) esp32.gpio.strapValue = 0x13;

if (esp32.mmuTablePro) {
    for (let p = 0; p < 64; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
}

try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
esp32.stopped = false;

if (esp32.uart?.[0]) {
    const u = esp32.uart[0];
    const orig = u.txUpdated?.bind?.(u);
    if (orig) {
        u.txUpdated = function() { orig(); if (this.txState !== 0) this.txComplete(); };
    }
}

// ---- UART capture ----
let uartLog = '';
let uartLines = [];
if (esp32.uart?.[0]) {
    esp32.uart[0].onTX = (byte) => {
        uartLog += String.fromCharCode(byte);
        if (uartLog.endsWith('\n')) {
            uartLines.push(uartLog.trim());
            if (uartLines.length > 200) uartLines = uartLines.slice(-200);
            uartLog = '';
        }
    };
}

// ---- Boot tracking ----
let maxPC = 0;
let stages = new Set();
let lastPC = -1;
let stuckCount = 0;
let cycles = 0;
const MAX = 100_000_000;

console.log(`\nSimulating ${MAX.toLocaleString()} cycles...\n`);

for (; cycles < MAX; cycles++) {
    esp32.step();
    if (esp32.clock) esp32.clock.tick?.();

    const pc = esp32.cores?.[0]?.PC;
    if (pc !== undefined) {
        if (pc === lastPC) stuckCount++; else stuckCount = 0;
        lastPC = pc;
        if (pc > maxPC) maxPC = pc;
        if (pc >= 0x40020000) stages.add('iram_app');
        if (pc >= 0x40080000) stages.add('flash_app');
        if (pc >= 0x3f400000 && pc < 0x3f800000) stages.add('flash_cache');
    }

    if (cycles && cycles % 5_000_000 === 0) {
        process.stdout.write(`\r  Cyc ${(cycles/1e6).toFixed(0)}M  PC=0x${lastPC?.toString(16).padStart(8,'0')}  max=0x${maxPC.toString(16).padStart(8,'0')}`);
    }

    if (stuckCount > 1_000_000) break;
}

console.log(`\n\n=== Results ===`);
console.log(`Total cycles: ${cycles.toLocaleString()}`);
console.log(`Final PC:     0x${lastPC?.toString(16).padStart(8, '0')}`);
console.log(`Max PC seen:  0x${maxPC.toString(16).padStart(8, '0')}`);
console.log(`Stuck count:  ${stuckCount}`);
console.log(`Stages:       ${[...stages].join(', ') || 'bootrom'}`);
console.log(`\nUART output (${uartLines.length} lines):`);
uartLines.slice(-15).forEach(l => console.log(`  ${l}`));

// Summary
const bootromOk = uartLines.length > 0 || maxPC > 0x40020000;
const flashRead = stages.has('flash_cache');
const appReached = stages.has('flash_app') || maxPC >= 0x40080000;

console.log(`\n=== Boot Status ===`);
console.log(`  BootROM executed:    ${bootromOk ? 'YES' : 'NO'}`);
console.log(`  Flash read via MMU:  ${flashRead ? 'YES' : 'NO'}`);
console.log(`  User code reached:   ${appReached ? 'YES' : 'NO'}`);

if (appReached) {
    console.log(`\nVERDICT: ✅ BOOT SUCCESSFUL — CPU is executing user code`);
} else if (uartLines.some(l => l.includes('invalid header'))) {
    console.log(`\nVERDICT: ⚠ BootROM loaded and runs, but flash header is invalid`);
    console.log(`         (expected — needs a valid ESP32 bootloader binary in flash)`);
} else if (bootromOk) {
    console.log(`\nVERDICT: ⚠ BootROM running, waiting for hardware init`);
} else {
    console.log(`\nVERDICT: ❌ CPU did not boot`);
}
