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

#define EMAC_MAC_BASE  0x3ff69000
#define EMAC_DMA_BASE  0x3ff6a000
// Offsets from emac_mac_struct.h / emac_dma_struct.h (in-order fields)
#define MAC_CONFIG0    (EMAC_MAC_BASE + 0x0)
#define MAC_FF         (EMAC_MAC_BASE + 0x4)
#define MAC_MDIO_ADDR  (EMAC_MAC_BASE + 0x10)
#define MAC_MDIO_DATA  (EMAC_MAC_BASE + 0x14)
#define MAC_FC         (EMAC_MAC_BASE + 0x18)
#define MAC_INTS       (EMAC_MAC_BASE + 0x38)
#define MAC_INTMASK    (EMAC_MAC_BASE + 0x3C)
#define MAC_ADDR0_HI   (EMAC_MAC_BASE + 0x40)
#define MAC_ADDR0_LO   (EMAC_MAC_BASE + 0x44)
#define MAC_WDOGTO     (EMAC_MAC_BASE + 0x5C)
#define DMA_BUSMODE    (EMAC_DMA_BASE + 0x0)
#define DMA_TXPOLL     (EMAC_DMA_BASE + 0x4)
#define DMA_RXPOLL     (EMAC_DMA_BASE + 0x8)
#define DMA_RXBASE     (EMAC_DMA_BASE + 0xC)
#define DMA_TXBASE     (EMAC_DMA_BASE + 0x10)
#define DMA_OPMODE     (EMAC_DMA_BASE + 0x18)
#define DMA_INTEN      (EMAC_DMA_BASE + 0x1C)
#define DMA_TXCURRDESC (EMAC_DMA_BASE + 0x48)

void setup() {
  Serial.begin(115200);
  Serial.println("=== EMAC TEST ===\\n");
  bool pass = true;

  // MAC block
  REG_WRITE(MAC_CONFIG0, 0x00000101);   // rx + tx enable
  uint32_t v = REG_READ(MAC_CONFIG0);
  Serial.printf("[EMAC] gmacconfig readback=%x\\n", v);
  if (v != 0x00000101) pass = false;

  REG_WRITE(MAC_ADDR0_HI, 0x8000);
  REG_WRITE(MAC_ADDR0_LO, 0x12345678);
  v = REG_READ(MAC_ADDR0_LO);
  Serial.printf("[EMAC] addr0low readback=%x\\n", v);
  if (v != 0x12345678) pass = false;

  REG_WRITE(MAC_FF, 0x3);
  v = REG_READ(MAC_FF);
  if (v != 0x3) pass = false;

  REG_WRITE(MAC_MDIO_ADDR, 0x00010002); // phy addr 1, reg 2
  v = REG_READ(MAC_MDIO_ADDR);
  if (v != 0x00010002) pass = false;

  REG_WRITE(MAC_MDIO_DATA, 0xDEAD);
  v = REG_READ(MAC_MDIO_DATA);
  if (v != 0xDEAD) pass = false;

  REG_WRITE(MAC_INTS, 0x3);
  REG_WRITE(MAC_INTMASK, 0x3);
  v = REG_READ(MAC_INTMASK);
  if (v != 0x3) pass = false;

  REG_WRITE(MAC_WDOGTO, 0x800);
  v = REG_READ(MAC_WDOGTO);
  if (v != 0x800) pass = false;

  // DMA block
  REG_WRITE(DMA_BUSMODE, 0x00020001);   // software reset + fixed burst
  v = REG_READ(DMA_BUSMODE);
  Serial.printf("[EMAC] dmabusmode readback=%x\\n", v);
  if (v != 0x00020001) pass = false;

  REG_WRITE(DMA_TXBASE, 0x3FFE0000);
  v = REG_READ(DMA_TXBASE);
  if (v != 0x3FFE0000) pass = false;

  REG_WRITE(DMA_RXBASE, 0x3FFE1000);
  v = REG_READ(DMA_RXBASE);
  if (v != 0x3FFE1000) pass = false;

  REG_WRITE(DMA_OPMODE, 0x00000003);    // tx + rx start
  v = REG_READ(DMA_OPMODE);
  if (v != 0x00000003) pass = false;

  REG_WRITE(DMA_INTEN, 0x3F);
  v = REG_READ(DMA_INTEN);
  Serial.printf("[EMAC] dmain_en readback=%x\\n", v);
  if (v != 0x3F) pass = false;

  REG_WRITE(DMA_TXPOLL, 0x1);
  v = REG_READ(DMA_TXPOLL);
  if (v != 0x1) pass = false;

  REG_WRITE(DMA_TXCURRDESC, 0x3FFE0000);
  v = REG_READ(DMA_TXCURRDESC);
  if (v != 0x3FFE0000) pass = false;

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
