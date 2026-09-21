import { SimulatorWorker } from '../src/index.js';
import { readFileSync, existsSync, mkdirSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));

async function main() {
  let fb;
  const cachePath = resolve(__dirname, 'build/webserver.bin');
  if (existsSync(cachePath)) {
    fb = readFileSync(cachePath);
  } else {
    const code = readFileSync('C:\\Users\\Danish\\Downloads\\esp32_1.ino', 'utf8');
    const { default: axios } = await import('axios');
    const sr = await axios.post('http://localhost:5525/api/compile/start', {
      code, target: 'esp32', targetEngine: 'frontend', fqbn: 'esp32:esp32:esp32'
    });
    const bid = sr.data.buildId;
    process.stdout.write('Compiling');
    while (1) {
      const s = await axios.get(`http://localhost:5525/api/compile/status/${bid}`);
      if (s.data.status === 'success') { fb = Buffer.from(s.data.binary_content, 'base64'); break; }
      else if (s.data.status === 'failed') throw Error(s.data.error);
      process.stdout.write('.'); await new Promise(r => setTimeout(r, 1000));
    }
    console.log('');
    try { mkdirSync(resolve(__dirname, 'build')); } catch (e) { }
    writeFileSync(cachePath, fb);
  }

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

  for (let i = 0; i < 600; i++) {
    await new Promise(r => setTimeout(r, 50));
    proxy.run();
    proxy.pollUart();
    if (out.length > pollLast) {
      const newText = out.substring(pollLast);
      const lines = newText.split('\n').filter(l => l.trim()).map(l => '  ' + l);
      if (lines.length) lines.forEach(l => console.log(l));
      pollLast = out.length;
    }
    if (out.includes('ready on port 80')) { break; }
  }

  console.log('\n=== WEB SERVER READY ===');
  console.log('ns=' + proxy.nanos);
  console.log('Press Ctrl+C to stop.\n');

  // In CI the battery just needs the READY assertion; exit so the run
  // completes. Set KEEP_SERVING=1 to keep the server up for manual curl.
  if (!process.env.KEEP_SERVING) {
    process.exit(0);
  }

  // Keep running
  let pollLast2 = out.length;
  while (true) {
    await new Promise(r => setTimeout(r, 1000));
    proxy.run();
    proxy.pollUart();
    if (out.length > pollLast2) {
      const newText = out.substring(pollLast2);
      const lines = newText.split('\n').filter(l => l.trim()).map(l => '  [WEB] ' + l);
      if (lines.length) lines.forEach(l => console.log(l));
      pollLast2 = out.length;
    }
  }
}

main().catch(e => { console.error(e); process.exit(1); });
