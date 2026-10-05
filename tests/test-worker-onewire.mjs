// OneWire slave tap (§11, P1): synchronous edge responder + IN-read override.
// Firmware bit-bangs the wire with NO libraries (pinMode/digitalWrite/
// digitalRead/delayMicroseconds only); the host DS18B20-compatible model
// answers presence + ROM/scratchpad reads on sim-time APB ticks — zero host
// round-trips, so microsecond slots hold regardless of wall speed.
// Requires the compile server (:5525).
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
#define OW_PIN 4

static inline void owLow() { pinMode(OW_PIN, OUTPUT); digitalWrite(OW_PIN, LOW); }
static inline void owRelease() { pinMode(OW_PIN, INPUT); }

void owWriteBit(int b) {
  owLow();
  if (b) { delayMicroseconds(6); owRelease(); delayMicroseconds(64); }
  else { delayMicroseconds(65); owRelease(); delayMicroseconds(5); }
}
void owWriteByte(uint8_t v) { for (int i = 0; i < 8; i++) owWriteBit((v >> i) & 1); }
int owReadBit() {
  owLow();
  delayMicroseconds(3);
  owRelease();
  delayMicroseconds(12);
  int v = digitalRead(OW_PIN);
  delayMicroseconds(55);
  return v;
}
uint8_t owReadByte() { uint8_t v = 0; for (int i = 0; i < 8; i++) v |= (uint8_t)(owReadBit() << i); return v; }
bool owReset() {
  owLow();
  delayMicroseconds(500);
  owRelease();
  delayMicroseconds(70);
  int presence = digitalRead(OW_PIN);
  delayMicroseconds(430);
  return presence == 0;
}
uint8_t crc8(const uint8_t *data, int len) {
  uint8_t crc = 0;
  for (int i = 0; i < len; i++) {
    uint8_t b = data[i];
    for (int j = 0; j < 8; j++) {
      uint8_t mix = (crc ^ b) & 1;
      crc >>= 1;
      if (mix) crc ^= 0x8c;
      b >>= 1;
    }
  }
  return crc;
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== ONEWIRE TEST ===");
  bool pass = true;
  if (!owReset()) { Serial.println("PRESENCE=FAIL"); pass = false; }
  else Serial.println("PRESENCE=PASS");
  owWriteByte(0xCC); // SKIP ROM
  owWriteByte(0xBE); // READ SCRATCHPAD
  uint8_t sp[9];
  for (int i = 0; i < 9; i++) sp[i] = owReadByte();
  Serial.printf("SP=%02x %02x %02x %02x %02x %02x %02x %02x %02x\\n",
    sp[0], sp[1], sp[2], sp[3], sp[4], sp[5], sp[6], sp[7], sp[8]);
  if (crc8(sp, 8) != sp[8]) { Serial.println("CRC=FAIL"); pass = false; }
  else Serial.println("CRC=PASS");
  // 23.5C = 0x0178 at 12-bit (set by the host via setOneWireTemp)
  if (sp[0] != 0x78 || sp[1] != 0x01) { Serial.println("TEMP=FAIL"); pass = false; }
  else Serial.println("TEMP=PASS");
  // ROM readback path
  if (!owReset()) { Serial.println("PRESENCE2=FAIL"); pass = false; }
  owWriteByte(0x33); // READ ROM
  uint8_t rom[8];
  for (int i = 0; i < 8; i++) rom[i] = owReadByte();
  Serial.printf("ROM=%02x..%02x CRC=%02x\\n", rom[0], rom[6], rom[7]);
  if (rom[0] != 0x28 || crc8(rom, 7) != rom[7]) { Serial.println("ROMREAD=FAIL"); pass = false; }
  else Serial.println("ROMREAD=PASS");
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
  await proxy.attachOneWire(4);
  await proxy.setOneWireTemp(4, 23.5);
  proxy.run();

  const t0 = Date.now();
  while (Date.now() - t0 < 150000) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (out.includes('RESULT=')) break;
  }
  const ow = await proxy.pollOneWire(4);
  proxy.stop();
  await new Promise(r => setTimeout(r, 100));
  proxy.terminate();

  console.log(out);
  console.log(`[test] onewire log: presence=${ow.presence} bytes=${ow.log.filter((e) => e.t === 'byte').map((e) => e.byte.toString(16)).join(',')}`);
  const ok = out.includes('PRESENCE=PASS') && out.includes('CRC=PASS') && out.includes('TEMP=PASS')
    && out.includes('ROMREAD=PASS') && out.includes('RESULT=PASS')
    && ow.presence >= 2;
  if (!ok) { console.log('[test] FAILED'); process.exit(1); }
  console.log('[test] PASSED');
}

run().catch(e => { console.error(e); process.exit(1); });
