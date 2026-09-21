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
#include <mbedtls/bignum.h>

static void mpi_set_str(mbedtls_mpi *m, const char *s) {
  if (mbedtls_mpi_read_string(m, 10, s) != 0) {
    Serial.println("MPI_READ_STRING=FAIL");
  }
}

void setup() {
  Serial.begin(115200);
  Serial.println("=== RSA TEST ===\\n");
  mbedtls_mpi base, exp, mod, res, expected;
  mbedtls_mpi_init(&base); mbedtls_mpi_init(&exp);
  mbedtls_mpi_init(&mod); mbedtls_mpi_init(&res);
  mbedtls_mpi_init(&expected);

  mpi_set_str(&base, "5");
  mpi_set_str(&exp, "65537");
  mpi_set_str(&mod, "11");
  mpi_set_str(&expected, "3");
  int err = mbedtls_mpi_exp_mod(&res, &base, &exp, &mod, NULL);
  bool expOk = (err == 0) && (mbedtls_mpi_cmp_mpi(&res, &expected) == 0);
  Serial.print("RSA_EXP1="); Serial.println(expOk ? "PASS" : "FAIL");

  mpi_set_str(&base, "3");
  mpi_set_str(&exp, "7");
  mpi_set_str(&mod, "17");
  mpi_set_str(&expected, "11");
  err = mbedtls_mpi_exp_mod(&res, &base, &exp, &mod, NULL);
  bool mulOk = (err == 0) && (mbedtls_mpi_cmp_mpi(&res, &expected) == 0);
  Serial.print("RSA_EXP2="); Serial.println(mulOk ? "PASS" : "FAIL");

  mbedtls_mpi_free(&base); mbedtls_mpi_free(&exp);
  mbedtls_mpi_free(&mod); mbedtls_mpi_free(&res);
  mbedtls_mpi_free(&expected);

  bool pass = expOk && mulOk;
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

  for (let i = 0; i < 150; i++) {
    await new Promise(r => setTimeout(r, 200));
    proxy.pollUart();
    if (proxy.nanos > 100000000 && out.includes('ALL TESTS PASSED')) break;
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
