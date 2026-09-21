import { ESP32 } from '../src/peripherals/esp32/esp32.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const flashBytes = readFileSync(resolve(__dirname, 'compiled-wifitest.bin'));
const romBytes = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

const DIV_STEP = 10105001;

async function main() {
  // JS chip - run to step just AFTER the divergence to see the state
  const cJS = new ESP32({ flashSizeMB: 4, mmuPages: 128, strapValue: 0x13 });
  cJS.flash.fill(0xff);
  cJS.flash.set(flashBytes);
  cJS.loadROM(romBytes);
  cJS.cores[1].enabled = true;
  cJS.reset();
  for (let i = 0; i < DIV_STEP; i++) { cJS.step(); }

  // Read the instruction and literal at 0x40000480 and 0x40000414
  const getByte = (addr) => {
    const reg = cJS.mapAddress(addr, 1); // core 1
    return reg.readUint8(addr);
  };

  // Instruction bytes at 0x40000480
  const b0 = getByte(0x40000480);
  const b1 = getByte(0x40000481);
  const b2 = getByte(0x40000482);
  const opcode = b0 | (b1 << 8) | (b2 << 16);
  console.log(`Instruction at 0x40000480: bytes=[0x${b0.toString(16)},0x${b1.toString(16)},0x${b2.toString(16)}] opcode=0x${opcode.toString(16).padStart(6,'0')}`);

  // Literal at 0x40000414
  const lit0 = getByte(0x40000414);
  const lit1 = getByte(0x40000415);
  const lit2 = getByte(0x40000416);
  const lit3 = getByte(0x40000417);
  const literal = lit0 | (lit1 << 8) | (lit2 << 16) | (lit3 << 24);
  console.log(`Literal at 0x40000414: 0x${literal.toString(16).padStart(8,'0')}`);
  
  // What's at 0x4000fda0?
  const fb0 = getByte(0x4000fda0);
  const fb1 = getByte(0x4000fda1);
  const fb2 = getByte(0x4000fda2);
  const fb3 = getByte(0x4000fda3);
  console.log(`Jump target 0x4000fda0: bytes=[0x${fb0.toString(16)},0x${fb1.toString(16)},0x${fb2.toString(16)},0x${fb3.toString(16)}]`);
  
  // Check INT_LEVEL for core 1
  console.log(`C1 INT_LEVEL [231] = 0x${cJS.cores[1].specialRegisters[231].toString(16)}`);

  // Check what PS register says for C1
  console.log(`C1 PS_REGISTER [4] = 0x${cJS.cores[1].specialRegisters[4].toString(16)}`);
  console.log(`C1 PS_INTLEVEL = ${15 & cJS.cores[1].specialRegisters[4]}`);
  console.log(`C1 PS_EXCM = ${(cJS.cores[1].specialRegisters[4] >> 3) & 1}`);
  console.log(`C1 PS_UM = ${(cJS.cores[1].specialRegisters[4] >> 4) & 1}`);
  console.log(`C1 PS_WOE = ${(cJS.cores[1].specialRegisters[4] >> 5) & 1}`);
  console.log(`C1 PS_OWB = ${(cJS.cores[1].specialRegisters[4] >> 6) & 7}`);

  // Check ExcCause
  console.log(`C1 EXCCAUSE [3] (before step at divergence) = 0x${cJS.cores[1].specialRegisters[3].toString(16)}`);

  // Now step once and see PC and EXCCAUSE
  cJS.step();
  console.log(`\n=== After step at divergence ===`);
  console.log(`C1 PC = 0x${cJS.cores[1].PC.toString(16)}`);
  console.log(`C1 EXCCAUSE [3] = 0x${cJS.cores[1].specialRegisters[3].toString(16)}`);
  console.log(`C1 INT_STATUS [20] = 0x${cJS.cores[1].specialRegisters[20].toString(16)}`);
  console.log(`C1 PS_REGISTER [4] = 0x${cJS.cores[1].specialRegisters[4].toString(16)}`);
  console.log(`C1 INT_LEVEL [231] = 0x${cJS.cores[1].specialRegisters[231].toString(16)}`);
  console.log(`C1 PS_INTLEVEL = ${15 & cJS.cores[1].specialRegisters[4]}`);
  console.log(`C1 PS_EXCM = ${(cJS.cores[1].specialRegisters[4] >> 3) & 1}`);
  console.log(`C1 PS_UM = ${(cJS.cores[1].specialRegisters[4] >> 4) & 1}`);
  console.log(`C1 pendingInterrupts = ${cJS.cores[1].pendingInterrupts}`);
  
  // Check what core 0 does during this step
console.log(`\nC0 PC = 0x${cJS.cores[0].PC.toString(16)}`);
console.log(`C0 pendingInterrupts = ${cJS.cores[0].pendingInterrupts}`);
console.log(`C0 INT_ENABLE [226] = 0x${cJS.cores[0].specialRegisters[226].toString(16)}`);
console.log(`C0 CLOCK_CONFIG [228] = 0x${cJS.cores[0].specialRegisters[228].toString(16)}`);
console.log(`C0 CLOCK_EVENT = ${cJS.cores[0].specialRegisters[226] & (cJS.cores[0].specialRegisters[228] | 0x4000)}`);

// Check if C0's step triggers anything for C1
// Run one more step to see C0's effect
cJS.step();
console.log(`\n=== After second step ===`);
console.log(`C0 PC = 0x${cJS.cores[0].PC.toString(16)}`);
console.log(`C1 PC = 0x${cJS.cores[1].PC.toString(16)}`);
console.log(`C1 INT_ENABLE = 0x${cJS.cores[1].specialRegisters[226].toString(16)}`);
console.log(`C1 pendingInterrupts = ${cJS.cores[1].pendingInterrupts}`);
}

main().catch(e => console.error(e));
