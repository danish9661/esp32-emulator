import { SimulatorWorker } from '../src/index.js';
import axios from 'axios';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function compile(code) {
  const r = await axios.post('http://localhost:5525/api/compile/start', { code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32' });
  while (true) {
    const s = await axios.get(`http://localhost:5525/api/compile/status/${r.data.buildId}`);
    if (s.data.status === 'success') return s.data.binary_content;
    if (s.data.status === 'failed') throw new Error('Compile failed: ' + s.data.error);
    await new Promise(r => setTimeout(r, 1000));
  }
}

const firmware = `
#include <soc/soc.h>
#include <soc/uhci_reg.h>

void setup() {
  Serial.begin(115200);
  Serial.println("=== UHCI TEST ===\\n");
  bool pass = true;

  // UHCI0 (0x3FF54000)
  REG_WRITE(UHCI_CONF0_REG(0), 0x00000007);
  uint32_t v = REG_READ(UHCI_CONF0_REG(0));
  Serial.printf("[UHCI] conf0(0) readback=%x\\n", v);
  if (v != 0x00000007) pass = false;

  REG_WRITE(UHCI_DMA_OUT_LINK_REG(0), 0x3FF80010);
  v = REG_READ(UHCI_DMA_OUT_LINK_REG(0));
  Serial.printf("[UHCI] out_link(0) readback=%x\\n", v);
  if (v != 0x3FF80010) pass = false;

  REG_WRITE(UHCI_DMA_IN_LINK_REG(0), 0x3FF80100);
  v = REG_READ(UHCI_DMA_IN_LINK_REG(0));
  if (v != 0x3FF80100) pass = false;

  REG_WRITE(UHCI_INT_ENA_REG(0), 0x000001FF);
  v = REG_READ(UHCI_INT_ENA_REG(0));
  Serial.printf("[UHCI] int_ena(0) readback=%x\\n", v);
  if (v != 0x000001FF) pass = false;

  REG_WRITE(UHCI_INT_CLR_REG(0), 0xFFFFFFFF);   // clears RAW/ST, no crash
  REG_WRITE(UHCI_CONF1_REG(0), 0x3);
  v = REG_READ(UHCI_CONF1_REG(0));
  if (v != 0x3) pass = false;

  REG_WRITE(UHCI_PKT_THRES_REG(0), 0x40);
  v = REG_READ(UHCI_PKT_THRES_REG(0));
  if (v != 0x40) pass = false;

  // UHCI1 (0x3FF4C000)
  REG_WRITE(UHCI_CONF0_REG(1), 0x00000003);
  v = REG_READ(UHCI_CONF0_REG(1));
  Serial.printf("[UHCI] conf0(1) readback=%x\\n", v);
  if (v != 0x00000003) pass = false;

  REG_WRITE(UHCI_DMA_IN_LINK_REG(1), 0x3FF80200);
  v = REG_READ(UHCI_DMA_IN_LINK_REG(1));
  if (v != 0x3FF80200) pass = false;

  REG_WRITE(UHCI_STATE0_REG(1), 0xABCD);
  v = REG_READ(UHCI_STATE0_REG(1));
  if (v != 0xABCD) pass = false;

  REG_WRITE(UHCI_Q0_WORD0_REG(1), 0xDEADBEEF);
  v = REG_READ(UHCI_Q0_WORD0_REG(1));
  if (v != 0xDEADBEEF) pass = false;

  Serial.print("RESULT="); Serial.println(pass ? "PASS" : "FAIL");
  Serial.println("\\n=== ALL TESTS PASSED ===");
}
void loop() { delay(1000); }
`;

async function run() {
  const b64 = await compile(firmware);
  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(new Uint8Array(Buffer.from(b64, 'base64').toString('binary').split('').map(c => c.charCodeAt(0))));
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));

  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);

  await proxy.init('ESP32', { flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 5000000 }, flash, rom);
  proxy.run();

  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 50000000 && out.includes('ALL TESTS PASSED')) break;
  }
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  if (!out.includes('RESULT=PASS') || !out.includes('ALL TESTS PASSED')) {
    console.log('[test] FAILED'); process.exit(1);
  }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
