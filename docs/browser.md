# Browser usage

The package ships a pre-bundled `dist/` (ESM). To use it in a browser you need a
bundler (esbuild / Vite / webpack) to roll `dist/` into your app. The worker is
spawned with `new URL('./worker-entry.js', import.meta.url)`, so the bundler must
emit `worker-entry.js` next to the entry and preserve `import.meta.url` (our
build places it at `dist/sab/worker-entry.js`).

## Cross-origin isolation (required)

`SharedArrayBuffer` is only available in cross-origin-isolated contexts. Serve the
page with:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Without these headers the browser disables `SharedArrayBuffer`, and the worker
cannot share memory with the main thread (UART ring + RAM inspection stop
working). See [audit.md](./audit.md) for the rationale and why the SAB is kept.

## Vite example

```js
// vite.config.js
export default {
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  worker: { format: 'es' },
};
```

Then `import { SimulatorWorker } from 'esp32emu'` and use it as in the
[API examples](./api.md). The loader fetches the wasm/ROM over `http(s)`; in a
bundled browser app those URLs resolve relative to the worker entry, so ensure
`dist/engine` and `dist/rom` assets are served.
