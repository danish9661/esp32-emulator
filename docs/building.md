# Building

## WASM engine (Rust → wasm32)

Requires Rust with the `wasm32-unknown-unknown` target.

Windows:

```
powershell -File build-wasm.ps1
```

Linux/macOS:

```
cd engine-wasm
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/esp_engine_wasm.wasm ../src/engine/esp-xtensa/esp_engine_wasm.wasm
```

The wasm binary is git-tracked and committed with Rust changes. After a rebuild,
verify the new exports are actually present before testing
(`strings .../esp_engine_wasm.wasm | grep native_<name>`) — a failed build can
ship a stale binary.

## `dist/` bundle (esbuild)

```
npm install
npm run build
```

`build.mjs` bundles `src/index.js` (+ `worker-proxy.js`, `worker-entry.js`) into
`dist/`, preserving the `src/` layout so the worker's `import.meta.url` still
resolves `./worker-entry.js` and `../engine|../rom`. Binaries are copied into
`dist/engine` and `dist/rom`. The published package ships `dist/`.

## CI

`.github/workflows/release.yml` builds the wasm engine, runs the no-server tests,
builds `dist/`, and (on a `v*` tag) publishes to npm and creates a GitHub release.
