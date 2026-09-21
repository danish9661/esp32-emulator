import { ESP32 } from '../src/index.js';
import { readFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import axios from 'axios';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const BOOTROM_PATH = join(ROOT, 'rom', 'esp32-v3-rom.bin');
const WASM_PATH = join(ROOT, 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');

const SKETCH_START = 44; // template literal start line in test-worker-proxy-esp32.mjs
const SKETCH_END = 236;

async function compileFirmware() {
  const testSrc = readFileSync(join(__dirname, 'test-worker-proxy-esp32.mjs'), 'utf8');
  const lines = testSrc.split('\n');
  const code = lines.slice(SKETCH_START, SKETCH_END - 1).join('\n')
    .replace(/\\n/g, '\n').replace(/\\"/g, '"').replace(/\\`/g, '`');
  const startRes = await axios.post('http://localhost:5525/api/compile/start', {
    code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32'
  });
  const id = startRes.data.buildId || startRes.data.id;
  for (let i = 0; i < 120; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    const statRes = await axios.get(`http://localhost:5525/api/compile/status/${id}`);
    if (statRes.data.status === 'success') {
      const b64 = statRes.data.binary_content;
      const bin = Buffer.from(b64, 'base64');
      console.log(`[compile] success ${bin.length} bytes`);
      return bin;
    }
    if (statRes.data.status === 'error' || statRes.data.error) {
      throw new Error('compile error: ' + JSON.stringify(statRes.data).slice(0, 500));
    }
  }
  throw new Error('compile timeout');
}

function makeChip(flashBytes) {
  const rom = readFileSync(BOOTROM_PATH);
  const flash = new Uint8Array(4 * 1024 * 1024);
  flash.fill(0xff);
  flash.set(flashBytes);
  const esp32 = new ESP32({ flashSizeMB: 4, flash });
  esp32.loadROM(rom);
  if (esp32.gpio?.pins) {
    if (esp32.gpio.pins[0])  esp32.gpio.pins[0].inputValue = true;
    if (esp32.gpio.pins[2])  esp32.gpio.pins[2].inputValue = false;
    if (esp32.gpio.pins[12]) esp32.gpio.pins[12].inputValue = false;
    if (esp32.gpio.pins[15]) esp32.gpio.pins[15].inputValue = false;
  }
  esp32.reset();
  esp32.flash.set(flash);
  if (esp32.gpio) esp32.gpio.strapValue = 0x13;
  const mmuPages = Math.ceil(flashBytes.length / 65536);
  if (esp32.mmuTablePro) {
    for (let p = 0; p < mmuPages; p++) { esp32.mmuTablePro[p] = p; esp32.mmuTableApp[p] = p; }
  }
  try { esp32.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  if (esp32.uart?.[0]) {
    esp32.uart[0].onTX = (byte) => { process.stdout.write(String.fromCharCode(byte)); };
  }
  if (esp32.cores?.[1]) esp32.cores[1].enabled = true;
  return esp32;
}

async function bootChip(flashBytes) {
  const chip = makeChip(flashBytes);
  const wasmBytes = readFileSync(WASM_PATH);
  const loaded = await chip.loadWasm(wasmBytes, 'wasm');
  if (!loaded) throw new Error('loadWasm failed');
  chip.reset();
  const mmuPages = Math.ceil(flashBytes.length / 65536);
  if (chip.mmuTablePro) {
    for (let p = 0; p < mmuPages; p++) chip.mmuTablePro[p] = p;
    if (chip.mmuTableApp) for (let p = 0; p < mmuPages; p++) chip.mmuTableApp[p] = p;
  }
  if (chip.cores?.[1]) chip.cores[1].enabled = true;
  if (chip._wasmCores?.[1]) chip._wasmCores[1].enabled = true;
  if (chip.gpio) chip.gpio.strapValue = 0x13;
  if (chip.uart?.[0]) chip.uart[0].onTX = (byte) => { process.stdout.write(String.fromCharCode(byte)); };
  try { chip.cores[0].writeUint32(0x3ff5a104, 0x5aa5); } catch {}
  return chip;
}

async function main() {
  const phase = process.argv[2] || 'walk';
  if (!readFileSync) {}
  let flashBytes;
  try {
    flashBytes = readFileSync(join(__dirname, 'fw-proxy32.bin'));
  } catch {
    flashBytes = Buffer.from(await compileFirmware());
    writeFileSync(join(__dirname, 'fw-proxy32.bin'), flashBytes);
    console.log('[walk] compiled + saved fw-proxy32.bin');
  }

  if (phase === 'walk') {
    const chip = await bootChip(flashBytes);
    let lastPc = 0;
    let lastPcLines = [];
    let t0 = Date.now();
    const TARGET = 15100000;
    for (let step = 0; step < 20000000; step++) {
      chip.step();
      const c = chip.cycles;
      if (c === TARGET) chip._wasmLoader.setDebugLog(1);
      if (c > TARGET && (c & 0x7F) === 0) {
        const now = Date.now();
        if (now - t0 > 300) {
          console.log(`\n[FREEZE] cycles=${c} step=${step} pc0=0x${chip._wasmCores[0].PC.toString(16)} pc1=0x${chip._wasmCores[1].PC.toString(16)}`);
          console.log('[FREEZE] last ~30 [WASM] traces:');
          console.log(lastPcLines.slice(-30).join('\n'));
          process.exit(0);
        }
        t0 = now;
      }
      if (c > TARGET) {
        const pc = chip._wasmCores[0].PC;
        lastPc = pc;
        lastPcLines.push(`  c=${c} pc0=0x${pc.toString(16)} pc1=0x${chip._wasmCores[1].PC.toString(16)}`);
        lastPcLines = lastPcLines.slice(-100);
      }
    }
    console.log('no freeze in 20M steps');
  }

  if (phase === 'bp') {
    const chip = await bootChip(flashBytes);
    const FREEZE_FROM = parseInt(process.argv[3] || '0x40085cf0', 16);
    const FREEZE_TO = parseInt(process.argv[4] || '0x40085d20', 16);
    const ARM = parseInt(process.argv[5] || '14950000', 16);
    const DUMP_AR = process.argv[6] !== 'noar';
    let armed = false;
    let t0 = Date.now();
    let hit = false;
    for (let step = 0; step < 20000000; step++) {
      chip.step();
      const c = chip.cycles;
      if (c >= ARM && !armed) { armed = true; t0 = Date.now(); }
      if (c >= 19900000 && (c % 5000) === 0) {
        const ar = [];
        for (let i = 0; i < 16; i++) ar.push(`${i}:${(chip._wasmCores[0].AR(i) >>> 0).toString(16)}`);
        console.log(`[STALL] c=${c} pc0=0x${chip._wasmCores[0].PC.toString(16)} op=0x${(chip._wasmCores[0].debugOpcode >>> 0).toString(16)} ar=${ar.join(' ')}`);
      }
      if (armed && (c & 0x1F) === 0 && Date.now() - t0 > 400) {
        console.log(`\n[FREEZE] cycles=${c} pc0=0x${chip._wasmCores[0].PC.toString(16)} pc1=0x${chip._wasmCores[1].PC.toString(16)}`);
        process.exit(0);
      }
      if (armed && !hit) {
        const pc0 = chip._wasmCores[0].PC, pc1 = chip._wasmCores[1].PC;
        if (pc0 >= FREEZE_FROM && pc0 <= FREEZE_TO) {
          hit = true;
          console.log(`[BP-HIT] c=${c} pc0=0x${pc0.toString(16)} op0=0x${(chip._wasmCores[0].debugOpcode >>> 0).toString(16)} pc1=0x${pc1.toString(16)} op1=0x${(chip._wasmCores[1].debugOpcode >>> 0).toString(16)}`);
          if (DUMP_AR) {
            const ar = [];
            for (let i = 0; i < 16; i++) ar.push(`a${i}=0x${(chip._wasmCores[0].AR(i) >>> 0).toString(16)}`);
            console.log(`[AR0] ${ar.join(' ')}`);
          }
          for (let i = 0; i < 200; i++) {
            chip.step();
            const p0 = chip._wasmCores[0].PC, p1 = chip._wasmCores[1].PC;
            const line = `  [${i}] c=${chip.cycles} pc0=0x${p0.toString(16)} op0=0x${(chip._wasmCores[0].debugOpcode >>> 0).toString(16)} pc1=0x${p1.toString(16)} op1=0x${(chip._wasmCores[1].debugOpcode >>> 0).toString(16)}`;
            if (DUMP_AR && p0 >= FREEZE_FROM && p0 <= FREEZE_TO) {
              const ar = [];
              for (let r = 0; r < 16; r++) ar.push(`${r}:0x${(chip._wasmCores[0].AR(r) >>> 0).toString(16)}`);
              console.log(line);
              console.log(`      [AR] ${ar.join(' ')}`);
            } else {
              console.log(line);
            }
            if (chip.cycles > ARM + 100000) { console.log('[BP] sim advanced past freeze window'); break; }
          }
          process.exit(0);
        }
      }
    }
    console.log(`[END] final c=${chip.cycles} pc0=0x${chip._wasmCores[0].PC.toString(16)} pc1=0x${chip._wasmCores[1].PC.toString(16)}`);
    console.log('no bp hit in 20M steps');
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
