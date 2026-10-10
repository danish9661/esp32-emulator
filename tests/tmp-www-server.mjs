// Static server for browser benchmarking (SAB needs COOP/COEP).
// Usage: node tests/tmp-www-server.mjs [port]  (serves ./webdemo)
import { createServer } from 'http';
import { readFile } from 'fs/promises';
import { join, extname, normalize } from 'path';

const root = join(process.cwd(), 'webdemo');
const port = Number(process.argv[2] || 8901);
const types = {
  '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm',
  '.bin': 'application/octet-stream', '.gz': 'application/gzip',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json',
};

createServer(async (req, res) => {
  try {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (p.endsWith('/')) p += 'index.html';
    const f = join(root, normalize(p).replace(/^(\.\.[/\\])+/, ''));
    const body = await readFile(f);
    res.writeHead(200, {
      'Content-Type': types[extname(f)] || 'application/octet-stream',
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cache-Control': 'no-store',
    });
    res.end(body);
  } catch {
    res.writeHead(404);
    res.end('nope');
  }
}).listen(port, () => console.log('www on ' + port));
