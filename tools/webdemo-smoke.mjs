// Smoke test for the webdemo Pages site (used by .github/workflows/webdemo.yml).
//
// Serves webdemo/ locally, loads all three pages in headless Chromium and
// asserts: zero page errors, the demo catalogue renders, and the docs
// support matrix renders. Kept as a file (not `node -e`) so shell quoting
// can never mangle it again.
//
// Run:  node tools/webdemo-smoke.mjs   (after `npm install --no-save playwright-core`
//   + `npx playwright install --with-deps chromium`)

import { chromium } from 'playwright-core';
import { createServer } from 'http';
import { readFileSync, existsSync } from 'fs';
import { join, extname } from 'path';

const MIME = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.gz': 'application/gzip',
  '.wasm': 'application/wasm',
  '.bin': 'application/octet-stream',
};

const server = createServer((req, res) => {
  let p = decodeURIComponent(new URL(req.url, 'http://x').pathname).replace(/^\//, '');
  if (p === '' || p.endsWith('/')) p += 'index.html';
  const f = join('webdemo', p);
  if (!existsSync(f)) {
    res.writeHead(404);
    res.end('nf');
    return;
  }
  res.writeHead(200, { 'content-type': MIME[extname(f)] || 'application/octet-stream' });
  res.end(readFileSync(f));
});

await new Promise((r) => server.listen(8942, '127.0.0.1', r));
const b = await chromium.launch();
const fails = [];
try {
  for (const page of ['index.html', 'about.html', 'docs.html']) {
    const pg = await b.newPage();
    const errs = [];
    pg.on('pageerror', (e) => errs.push(String(e).slice(0, 150)));
    await pg.goto('http://127.0.0.1:8942/' + page);
    await pg.waitForLoadState('networkidle');
    await pg.waitForTimeout(800);
    if (page === 'index.html') {
      const n = await pg.evaluate('document.querySelectorAll(".demo-item").length');
      if (n < 53) fails.push('demo count ' + n);
      // New UI: filter box, category select, sketch modal wiring.
      const ui = await pg.evaluate('JSON.stringify({filter: !!document.getElementById("demoFilter"), cat: !!document.getElementById("demoCat"), sketch: !!document.getElementById("btnSketch"), copy: !!document.getElementById("btnCopyLog"), dl: !!document.getElementById("btnDlLog"), mips: !!document.getElementById("demoMips")})');
      const u = JSON.parse(ui);
      for (const [k, v] of Object.entries(u)) {
        if (!v) fails.push('missing #' + k);
      }
      // Filter actually filters: type "wifi", expect fewer visible items.
      await pg.fill('#demoFilter', 'wifi');
      await pg.waitForTimeout(300);
      const vis = await pg.evaluate('[...document.querySelectorAll(".demo-item")].filter((el) => !el.hidden).length');
      if (!(vis >= 1 && vis < n)) fails.push('filter broken (visible=' + vis + ' of ' + n + ')');
      await pg.fill('#demoFilter', '');
      await pg.waitForTimeout(300);
    }
    if (page === 'docs.html') {
      const m = await pg.evaluate('document.querySelectorAll("#supportTable tbody tr").length');
      if (m < 53) fails.push('matrix rows ' + m);
      const mt = await pg.evaluate('document.querySelectorAll("#mipsTable tbody tr").length');
      if (mt < 53) fails.push('mips rows ' + mt);
    }
    if (errs.length) fails.push(page + ': ' + errs.join(' | '));
    await pg.close();
  }
} finally {
  await b.close();
  server.close();
}
if (fails.length) {
  console.error('SMOKE FAIL:', fails.join('; '));
  process.exit(1);
}
console.log('webdemo smoke test passed');
