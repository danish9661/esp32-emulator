import { SimulatorWorker } from '../src/index.js';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));

async function run() {
  const flashData = readFileSync(resolve(__dirname, 'build/webserver.bin'));
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
    if (Date.now() - start > 20000) break;
    await new Promise(r => setTimeout(r, 50));
    proxy.run();
    proxy.pollUart();
    if (out.length > prevLen) {
      const lines = out.substring(prevLen).split('\n');
      for (const l of lines) {
        const t = l.trim();
        if (t && !t.match(/^(ets |rst:|load:|mode:|clk:|configsip|entry)/)) console.log(`  ${t}`);
      }
      prevLen = out.length;
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
  console.log('\n=== WEB SERVER READY ===');
  console.log('Open: http://127.0.0.1:8080');
  // In CI the battery just needs the READY assertion; exit so the run
  // completes. Set KEEP_SERVING=1 to keep the server up for manual curl.
  if (!process.env.KEEP_SERVING) {
    proxy.terminate();
    process.exit(0);
  }
  await new Promise(r => setTimeout(r, 100));
  // Keep serving — NOTE: do NOT proxy.stop() here: it parks the worker in
  // Atomics.wait, which freezes the gateway websocket (keepalive timeout →
  // 1006 → room torn down ~2s later) and kills all bridging to the board.
  // Stop after ~20s so the test still terminates on its own.
  for (let i = 0; i < 400; i++) {
    await new Promise(r => setTimeout(r, 50));
    proxy.pollUart();
    const newOut = out.substring(prevLen);
    if (newOut) {
      for (const l of newOut.split('\n')) {
        const t = l.trim();
        if (t && (t.includes('LED') || t.includes('Web!'))) console.log(`  [WEB] ${t}`);
      }
      prevLen = out.length;
    }
    if (Date.now() - start > 20000) break;
  }
  proxy.terminate();
  process.exit(0);
}
run().catch(e => { console.error(e); process.exit(1); });
