import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const rom = readFileSync(resolve(__dirname, 'rom/esp32-v3-rom.bin'));
const v = new DataView(rom.buffer, rom.byteOffset, rom.byteLength);
const val = v.getUint32(0x414, true);
console.log('Value at ROM[0x414] (LE): 0x' + val.toString(16).padStart(8, '0'));

// Also check what the two CPUs would load
// Core 0: next_pc = 0x4000bff0 + 3 = 0x4000bff3
// Core 1: next_pc = 0x40000480 + 3 = 0x40000483
function l32rLoad(next_pc, imm16) {
  const addr = (((next_pc >>> 2) + (0xffff0000 | imm16)) >>> 0) << 2;
  const off = addr - 0x40000000;
  const val = v.getUint32(off, true);
  console.log('L32R: next_pc=0x' + next_pc.toString(16) + ' imm16=0x' + imm16.toString(16) + ' addr=0x' + addr.toString(16) + ' ROMoff=0x' + off.toString(16) + ' val=0x' + val.toString(16));
}

// Core 1 instruction at 0x40000480: opcode 0xFFE521
l32rLoad(0x40000483, 0xFFE5);

// Core 0 instruction at 0x4000bff0: opcode 0x342030 - this is EXTUI, not L32R
// Let me also check what happens at the pre-core-0-divergence instruction
// Actually the core 0 PC at 0x4000bff0 was EXTUI, which doesn't load from memory
