import { createServer } from 'http';
import { createHash } from 'crypto';
import { spawn } from 'child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';

// Local replacement for the remote compile server (http://localhost:5525).
// Implements the same API backed by the local arduino-cli:
//   POST /api/compile/start   { code, target, targetEngine, fqbn }
//   GET  /api/compile/status/:id  -> { status: running|success|failed, binary_content? }
// Compiles with `arduino-cli` (client fqbn if ESP32, else esp32:esp32:esp32)
// and serves the factory flash image (merged.bin, bootloader+partitions+app).
// Results are cached by fqbn + sketch-content hash, so consecutive runs are fast.

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const CACHE_DIR = join(ROOT, 'tests', 'build', 'local-compile');
const DEFAULT_FQBN = 'esp32:esp32:esp32';
const PORT = Number(process.env.PORT || 5525);
const COMPILE_TIMEOUT_MS = 900_000;

// Only ESP32 (never C3/RV) targets, e.g. 'esp32:esp32:esp32',
// 'esp32:esp32:esp32:PSRAM=enabled', 'esp32:esp32:esp32cam'. The client
// fqbn is honored so tests can enable menu options (PSRAM); anything else
// falls back to the default (prevents arbitrary arduino-cli targets).
function sanitizeFqbn(fqbn) {
  if (typeof fqbn !== 'string') return DEFAULT_FQBN;
  if (!fqbn.startsWith('esp32:esp32:')) return DEFAULT_FQBN;
  if (!/^[A-Za-z0-9_:.,=+-]+$/.test(fqbn)) return DEFAULT_FQBN;
  return fqbn;
}

mkdirSync(CACHE_DIR, { recursive: true });

const jobs = new Map();
let compiling = null;
let queue = null;

function hash(code) {
  return createHash('sha256').update(code).digest('hex').slice(0, 16);
}

function compileOne({ id, code, fqbn }) {
  const dir = join(CACHE_DIR, id);
  const outDir = join(dir, 'out');
  const sketchDir = join(dir, 'sketch');
  mkdirSync(sketchDir, { recursive: true });
  writeFileSync(join(sketchDir, 'sketch.ino'), code);

  const job = jobs.get(id) || { status: 'running', log: '' };
  job.status = 'running';
  jobs.set(id, job);

  const p = spawn('arduino-cli', ['compile', '--fqbn', fqbn, '--output-dir', outDir, sketchDir], {
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let log = '';
  const timer = setTimeout(() => {
    p.kill('SIGKILL');
  }, COMPILE_TIMEOUT_MS);
  p.stdout.on('data', (d) => (log += d));
  p.stderr.on('data', (d) => (log += d));
  p.on('close', (codeOut) => {
    clearTimeout(timer);
    if (codeOut === 0) {
      const candidates = [
        join(outDir, 'sketch.ino.merged.bin'),
        join(outDir, 'sketch.ino.merged.bin'),
        join(outDir, 'sketch.ino.bin'),
      ];
      // arduino-cli output dir: <sketch>.ino.merged.bin (factory image)
      const merged = join(outDir, 'sketch.ino.merged.bin');
      const bin = join(outDir, 'sketch.ino.bin');
      if (existsSync(merged)) {
        const b64 = readFileSync(merged).toString('base64');
        job.status = 'success';
        job.binary_content = b64;
      } else if (existsSync(bin)) {
        job.status = 'success';
        job.binary_content = readFileSync(bin).toString('base64');
      } else {
        job.status = 'failed';
        job.error = 'No .bin output produced. Log:\n' + log;
      }
    } else {
      job.status = 'failed';
      job.error = `arduino-cli exited with ${codeOut}. Log:\n` + log.slice(-4000);
    }
    jobs.set(id, job);
    if (queue.length > 0) {
      const next = queue.shift();
      queue = [];
      compileOne(next);
    } else {
      queue = null;
    }
  });
}

function enqueue(id, code, fqbn) {
  if (queue) queue.push({ id, code, fqbn });
  else {
    queue = [];
    compileOne({ id, code, fqbn });
  }
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);

  // POST /api/compile/start
  if (req.method === 'POST' && url.pathname === '/api/compile/start') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let payload = {};
      try {
        payload = JSON.parse(body);
      } catch {}
      const code = typeof payload.code === 'string' ? payload.code : '';
      if (!code) {
        res.writeHead(400, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ error: 'missing code' }));
        return;
      }
      const fqbn = sanitizeFqbn(payload.fqbn);
      const id = hash(fqbn + '\n' + code);
      // Retry failed jobs: a transient toolchain failure (e.g. PyInstaller
      // extraction under disk pressure) used to poison the id forever,
      // failing every later test with that firmware instantly.
      const prev = jobs.get(id);
      if (!prev || prev.status === 'failed' || prev.status === 'error') {
        jobs.set(id, { status: 'running' });
        enqueue(id, code, fqbn);
      }
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ buildId: id }));
    });
    return;
  }

  // GET /api/compile/status/:id
  const m = url.pathname.match(/^\/api\/compile\/status\/([^/]+)$/);
  if (req.method === 'GET' && m) {
    const job = jobs.get(m[1]) || { status: 'failed', error: 'unknown build id' };
    res.writeHead(200, { 'content-type': 'application/json' });
    if (job.status === 'success') {
      res.end(JSON.stringify({ status: 'success', binary_content: job.binary_content }));
    } else if (job.status === 'failed' || job.status === 'error') {
      res.end(JSON.stringify({ status: 'failed', error: job.error || 'compile failed' }));
    } else {
      res.end(JSON.stringify({ status: 'running' }));
    }
    return;
  }

  if (url.pathname === '/') {
    res.writeHead(200, { 'content-type': 'text/plain' });
    res.end(`local compile server (arduino-cli, default ${DEFAULT_FQBN}), cache=${CACHE_DIR}, jobs=${jobs.size}`);
    return;
  }

  res.writeHead(404, { 'content-type': 'application/json' });
  res.end(JSON.stringify({ error: 'not found' }));
});

server.listen(PORT, () => {
  console.log(`[compile-server] listening on :${PORT} (arduino-cli, default ${DEFAULT_FQBN}, cache=${CACHE_DIR})`);
});