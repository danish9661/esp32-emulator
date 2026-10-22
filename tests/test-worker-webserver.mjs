import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { buildWebFirmware, fetchBoard, WEB_MARKER } from './web-firmware.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));

// End-to-end: boot the web-server firmware on the worker, then drive a real
// HTTP request through the gateway's static forward into the emulated board
// (the same path a browser/curl uses) and assert the served body.
async function run() {
  const flashData = await buildWebFirmware();
  const flash = new Uint8Array(new SharedArrayBuffer(flashData.length));
  flash.set(flashData);
  const rom = readFileSync(resolve(__dirname, '../rom/esp32-v3-rom.bin'));
  const proxy = new SimulatorWorker();
  let out = '';
  proxy._onUART = (b) => { out += String.fromCharCode(b); };
  proxy._onError = (e) => console.error('\n[test] Error:', e.message);
  await proxy.init('ESP32', {
    flashSizeMB: 4, mmuPages: 64, strapValue: 0x13, budget: 80000000,
    wifi: { ssid: 'TEST-AP', channel: 6 },
    macAddress: '24:0a:c4:12:34:56'
  }, flash, rom);
  console.log('[test] Web server running via worker...\n');
  const start = Date.now();
  let prevLen = 0;
  proxy.run();
  for (let i = 0; i < 4000; i++) {
    // Check exit conditions BEFORE the blocking proxy.run(): once the
    // firmware is idle (after "ready"), proxy.run() parks in Atomics.wait
    // and would otherwise never return, so these checks must run first.
    if (out.includes('ready on port 80')) break;
    if (Date.now() - start > 60000) break;
    await new Promise(r => setTimeout(r, 50));
    proxy.run();
    proxy.pollUart();
    if (out.length > prevLen) {
      const lines = out.substring(prevLen).split('\n');
      for (const l of lines) {
        const t = l.trim();
        if (t && !t.match(/^(ets |rst:|load:|mode:|clk:|configsip|entry|ho )/)) console.log(`  ${t}`);
      }
      prevLen = out.length;
    }
  }

  let pass = true;
  if (!out.includes('ready on port 80')) {
    console.log('[test] FAILED: web server never became ready');
    pass = false;
  }

  // Real HTTP through the gateway (board keeps running: do NOT stop the
  // worker before the request — a stopped worker freezes the bridge).
  let httpBytes = 0;
  if (pass) {
    const root = await fetchBoard('/', 15000);
    if (!root.ok) {
      console.log('[test] FAILED: HTTP GET / -> ' + root.error);
      pass = false;
    } else if (root.status !== 200 || !root.body.includes(WEB_MARKER)) {
      console.log(`[test] FAILED: HTTP GET / -> status=${root.status} marker=${root.body.includes(WEB_MARKER)} bytes=${root.bytes}`);
      pass = false;
    } else {
      httpBytes = root.bytes;
      console.log(`[HTTP] GET / -> ${root.status} ${root.bytes}B on :${root.port} marker=${WEB_MARKER}`);
    }
  }
  if (pass) {
    const led = await fetchBoard('/led', 15000);
    if (!led.ok || led.status !== 200 || !led.body.includes('led=1')) {
      console.log(`[test] FAILED: HTTP GET /led -> ${led.ok ? 'status=' + led.status + ' body=' + led.body.slice(0, 40) : led.error}`);
      pass = false;
    } else {
      console.log(`[HTTP] GET /led -> ${led.status} ${led.bytes}B body=led=1`);
    }
  }

  // Stop the worker so it exits runSimChunk and returns to commandLoop,
  // otherwise getPcapData() busy-waits forever for a command the running
  // worker never processes.
  proxy.stop();
  const pcap = await Promise.race([
    proxy.getPcapData(),
    new Promise(r => setTimeout(() => r(new Uint8Array()), 1000)),
  ]).catch(() => new Uint8Array());
  if (pcap.length > 0) console.log('[PCAP] saved (' + pcap.length + ' bytes)');

  if (!pass) {
    try { proxy.terminate(); } catch (e) { }
    console.log('[test] FAILED');
    process.exit(1);
  }

  console.log('\n=== WEB SERVER READY ===');
  console.log('served ' + httpBytes + ' bytes from the emulated board');
  // In CI the battery just needs the assertions above; exit so the run
  // completes. Set KEEP_SERVING=1 to keep the server up for manual curl.
  if (!process.env.KEEP_SERVING) {
    proxy.terminate();
    console.log('[test] PASSED');
    process.exit(0);
  }
  await new Promise(r => setTimeout(r, 100));
  // Keep serving — NOTE: do NOT proxy.stop() here: it parks the worker in
  // Atomics.wait, which freezes the gateway websocket (keepalive timeout →
  // 1006 → room torn down ~2s later) and kills all bridging to the board.
  // Stop after ~20s so the test still terminates on its own.
  proxy.run();
  for (let i = 0; i < 400; i++) {
    await new Promise(r => setTimeout(r, 50));
    proxy.pollUart();
    const newOut = out.substring(prevLen);
    if (newOut) {
      for (const l of newOut.split('\n')) {
        const t = l.trim();
        if (t && (t.includes('WEB!') || t.includes('Web!'))) console.log(`  [WEB] ${t}`);
      }
      prevLen = out.length;
    }
    if (Date.now() - start > 60000) break;
  }
  proxy.terminate();
  process.exit(0);
}
run().catch(e => { console.error(e); process.exit(1); });
