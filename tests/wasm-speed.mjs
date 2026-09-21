// Simple WASM speed test
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load our actual engine WASM
const WASM_PATH = join(__dirname, '..', 'src', 'engine', 'esp-xtensa', 'esp_engine_wasm.wasm');
const wasmBytes = readFileSync(WASM_PATH);

const mod = new WebAssembly.Module(wasmBytes);
const imports = {
  env: {
    memory: new WebAssembly.Memory({ initial: 300, maximum: 300 }),
    mmio_read: () => 0,
    mmio_write: () => {},
    map_read: () => 0,
    map_write: () => {},
    map_address: () => 0,
    on_unknown_inst: () => {},
    on_break: () => {},
    write_watchpoint: () => 0,
    trace_mem_write: () => {},
    trace_entry: () => {},
    trace_return: () => {},
    get_cpu_ticks: () => 0,
    get_cpu_cycles: () => 0,
    get_sim_freq: () => 0,
    get_cpu_freq: () => 0,
    ccompare_schedule: () => {},
    ccompare_unschedule: () => {},
    efuse_cmd_schedule: () => {},
    is_window_inst: () => 0,
    has_breakpoint: () => 0,
    abort: () => {},
    js_random: () => Math.random(),
    js_apb_ticks: () => 0,
    js_clock_nanos: () => 0,
    js_interrupt: () => {},
    js_log_u32: () => {},
    js_log_str: () => {},
  }
};
const t0 = process.hrtime.bigint();
const inst = new WebAssembly.Instance(mod, imports);
const t1 = process.hrtime.bigint();
console.log('Instantiation: ' + Number(t1-t0)/1e6 + 'ms');

const exports = inst.exports;

// Time core_init
const t2 = process.hrtime.bigint();
exports.native_peripheral_init();
exports.core_init(0, 0, 0);
const t3 = process.hrtime.bigint();
console.log('Init: ' + Number(t3-t2)/1e6 + 'ms');

// Time core_step_batch with no real memory
const N = 100000;
const t4 = process.hrtime.bigint();
const steps = exports.core_step_batch(0, N);
const t5 = process.hrtime.bigint();
const us = Number(t5 - t4) / 1000;
console.log(`core_step_batch(0, ${N}) = ${steps} steps in ${us.toFixed(0)}us = ${(us/N).toFixed(3)}us/inst`);
console.log(`Rate: ${(N / us * 1e6 / 1000).toFixed(0)}K insts/s`);
