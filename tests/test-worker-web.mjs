import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { buildWebFirmware } from './web-firmware.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Boots the shared web-server firmware and waits for it to bring the HTTP
// server up ("ready on port 80"). test-worker-webserver.mjs then drives the
// real HTTP request through the gateway.
async function main() {
  const fb = await buildWebFirmware();
  console.log('[test] firmware ' + fb.length + ' bytes');

  const flash = new Uint8Array(new SharedArrayBuffer(4 * 1024 * 1024));
  flash.set(fb);
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();

  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[Error]', e.message);

  await proxy.init('ESP32', {
    flashSizeMB: 4,
    mmuPages: 64,
    strapValue: 0x13,
    budget: 80000000,
    wifi: { ssid: 'TEST-AP', channel: 6 },
    macAddress: '24:0a:c4:12:34:56',
  }, flash, rom);

  proxy.run();
  let pollLast = 0;
  const t0 = Date.now();

  while (Date.now() - t0 < 120000) {
    await new Promise(r => setTimeout(r, 50));
    proxy.run();
    proxy.pollUart();
    if (out.length > pollLast) {
      const lines = out.substring(pollLast).split('\n').filter(l => l.trim()).map(l => '  ' + l.trim());
      if (lines.length) lines.forEach(l => console.log(l));
      pollLast = out.length;
    }
    if (out.includes('ready on port 80')) break;
    if (out.includes('WEB=FAIL')) break;
  }

  const ready = out.includes('ready on port 80');
  if (!ready) {
    console.log('\n[test] FAILED (server never reported "ready on port 80")');
    try { proxy.stop(); proxy.terminate(); } catch (e) { }
    process.exit(1);
  }

  console.log('\n=== WEB SERVER READY ===');
  console.log('ns=' + proxy.nanos);

  // In CI the battery just needs the READY assertion; exit so the run
  // completes. Set KEEP_SERVING=1 to keep the server up for manual curl.
  if (!process.env.KEEP_SERVING) {
    try { proxy.stop(); proxy.terminate(); } catch (e) { }
    console.log('[test] PASSED');
    process.exit(0);
  }

  console.log('Press Ctrl+C to stop.\n');
  let pollLast2 = out.length;
  for (;;) {
    await new Promise(r => setTimeout(r, 1000));
    proxy.run();
    proxy.pollUart();
    if (out.length > pollLast2) {
      const lines = out.substring(pollLast2).split('\n').filter(l => l.trim()).map(l => '  [WEB] ' + l.trim());
      if (lines.length) lines.forEach(l => console.log(l));
      pollLast2 = out.length;
    }
  }
}

main().catch(e => { console.error(e); process.exit(1); });
