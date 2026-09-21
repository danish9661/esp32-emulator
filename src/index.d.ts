// Type definitions for esp32emu
// (hand-authored; mirrors the public exports from src/index.js)

/** AI-Thinker ESP32-CAM board id ('esp32-cam'). */
export const ESP32_CAM_BOARD_ID: string;
/** AI-Thinker ESP32-CAM pinout (camera + LEDs). */
export const ESP32_CAM_PINS: Record<string, number>;
/** ESP32-CAM memory defaults applied when `board: 'esp32-cam'`. */
export const ESP32_CAM_DEFAULTS: Record<string, unknown>;
/** Merge a board preset into a chip config (explicit keys win; throws on unknown board). */
export function applyBoardPreset(config?: Record<string, unknown>): Record<string, unknown>;

export interface WifiStats {
  state: number;
  txFrames: number;
  txBytes: number;
  rxFrames: number;
  rxBytes: number;
  probeRequestCount: number;
  connectedClients: number;
  ip: string;
  portForward: string;
  errorMessage: string;
}

export interface SimulatorConfig {
  /** Board preset: 'esp32' (default) or 'esp32-cam' (AI-Thinker: 4MB flash + 4MB PSRAM). Explicit sizes always win over preset defaults. */
  board?: string;
  flashSizeMB?: number;
  flashSize?: number;
  psramSize?: number;
  psramSizeMB?: number;
  mmuPages?: number;
  strapValue?: number;
  budget?: number;
  simMode?: 'tight' | 'chunked' | 'auto';
  chunkSize?: number;
  progressInterval?: number;
  cpuFrequency?: number | 'auto' | 'max';
  macAddress?: string | number[];
  analogInputs?: Record<number, number>;
  /** Virtual SD card: true/default 16MB, { sizeMB?, blocks?, image?, type? }, false = no card. type 'mmc' = eMMC card (MMC probe path). */
  sdCard?: boolean | { sizeMB?: number; blocks?: number; image?: Uint8Array; type?: 'sd' | 'mmc' };
  /** Touch pad readings (pad 0-9 -> raw count). Default 1000 (untouched). */
  touchInputs?: Record<number, number>;
  /** DAC channel voltages (0/1 -> volts). dacWrite() updates these (ADC loopback). */
  dacOutputs?: Record<number, number>;
  /** Virtual-camera frame size in bytes (finite sensor frame per capture, e.g. 160*120*2 for QQVGA RGB565). */
  camFrameBytes?: number;
  wifi?: boolean | { ssid?: string; bssid?: string | number[]; channel?: number; room?: string };
  [key: string]: unknown;
}

export class SimulatorWorker {
  constructor();
  /** Called per UART byte produced by the running firmware. */
  _onUART?: (byte: number) => void;
  _onError?: (err: Error) => void;
  _onReady?: () => void;

  simMode: 'tight' | 'chunked' | 'auto';
  chunkSize: number;
  cpuFrequency: number | 'auto' | 'max';
  budget: number;

  readonly running: boolean;
  readonly pc: number;
  readonly pc1: number;
  readonly nanos: number;
  readonly stuck: number;
  readonly idle: boolean;
  readonly ready: boolean;
  readonly macAddress: string | null;
  readonly board: string;
  readonly psramType: string | null;
  readonly flashSizeMB: number | null;
  readonly psramSizeMB: number | null;
  readonly wifiState: number;
  readonly wifiTxFrames: number;
  readonly wifiTxBytes: number;
  readonly wifiRxFrames: number;
  readonly wifiRxBytes: number;
  readonly wifiProbes: number;

  /** Direct SharedArrayBuffer views into emulated memory (flash/iram/dataMem/rtcFastMem/...). */
  memory: Record<string, Uint8Array>;

  init(
    chipType: string,
    config?: SimulatorConfig,
    flash?: Uint8Array | null,
    rom?: Uint8Array | null,
  ): Promise<void>;
  run(): void;
  stop(): void;
  terminate(): void;
  seedMMU(offset?: number): Promise<void>;
  reset(): Promise<void>;
  /**
   * Tap the RESET (EN) button: full chip reset like pulling CHIP_PU low
   * (digital + RTC state cleared, flash/RTC-memory preserved). Mid-run safe.
   */
  pressResetButton(): Promise<void>;
  /**
   * Hold/release the BOOT button (GPIO0 strapping): drives GPIO0's live
   * input level + the BOOT strap bit, so a subsequent reset samples
   * download mode like a real dev board. Mid-run safe.
   */
  pressBootButton(held: boolean): Promise<void>;
  writeUint32(addr: number, value: number): Promise<void>;
  readMemory(addr: number, bytes?: number): Promise<Uint8Array>;
  readMmio(hid: number, addr: number, size?: number): Promise<number>;
  step(count?: number): Promise<void>;
  getPcapData(): Promise<Uint8Array>;
  getFfiCounts(): Promise<number[]>;
  getWifiStats(): Promise<WifiStats | null>;
  /** Drain the UART ring buffer (call on a timer while running). */
  pollUart(): void;
  /**
   * Drive a GPIO input level at runtime (host-driven pin toggle).
   * Routes through the native GPIO matrix to PCNT/RMT/etc. inputs.
   */
  setPinInput(pin: number, level: number): Promise<void>;
  setTouchInput(pad: number, count: number): Promise<void>;
  setVoltageMv(mv: number): Promise<void>;
  feedI2SRX(sample: number): Promise<void>;
  sendTwaiFrame(id: number, data: number[] | Uint8Array, opts?: { ext?: boolean; rtr?: boolean }): Promise<void>;
  getTwaiTx(): Promise<{ count: number; id: number; ext: boolean; rtr: boolean; dlc: number; data: number[] } | null>;
}

export class MultiSimulator {
  constructor();
  nodes: Array<{ chip: ESP32; [key: string]: unknown }>;

  addNode(ChipClass: typeof ESP32, config?: Record<string, unknown>): void;
  loadWasm(nodeIdx: number, wasmBytes: Uint8Array | ArrayBuffer, mode?: string): Promise<void>;
  step(): void;
  run(steps: number): void;
  chip(nodeIdx: number): ESP32 | undefined;
  cycles(nodeIdx: number): number;
  addNodeParallel(
    chipType: string,
    config?: Record<string, unknown>,
    flash?: Uint8Array | null,
    rom?: Uint8Array | null,
    wasmBinary?: Uint8Array | null,
  ): Promise<void>;
  runParallel(opts?: { onPoll?: (() => void) | null; pollInterval?: number }): Promise<void>;
  stepParallel(batchSize: number): void;
  getUART(nodeIdx: number): Uint8Array | undefined;
  comparePC(nodeIdxA: number, nodeIdxB: number, coreIdx?: number): boolean;
  comparePhysRegs(nodeIdxA: number, nodeIdxB: number, coreIdx?: number, count?: number): boolean;
  compareAll(nodeIdxA: number, nodeIdxB: number, coreIdx?: number): boolean;
  debugView(nodeIdx: number): unknown;
  debugState(nodeIdx: number): unknown;
  debugDiffs(nodeIdxA: number, nodeIdxB: number): unknown;
  formatReport(diffs: unknown): string;
  compareDebug(nodeIdxA: number, nodeIdxB: number, customFormatter?: (diffs: unknown) => string): string;
}

export class ESP32 {
  constructor(cpuVal?: Record<string, unknown>);
  board: string;
  flash: Uint8Array;
  chipROM: ReadonlyMemory;
  iram: Memory;
  dataMem: Memory;
  rtcFastMem: Memory;
  psram: Uint8Array;

  loadROM(cpuVal: Uint8Array): void;
  loadWasm(wasmBytes: Uint8Array | ArrayBuffer, enabled?: string): Promise<void>;
  reset(): void;
  step(): void;
  mapAddress(cpuVal: number, coreIdx?: number): Uint8Array | null;
  interrupt(cpuVal: number, tmpVal?: boolean, idxVal?: number): void;
}

/** Facade exposing WASM core state (PC, regs, ccompare) to JS. Does NOT execute. */
export class XtensaCore {
  constructor(...args: unknown[]);
}

// ---- Memory primitives ----
export class Memory {
  constructor(data: Uint8Array, base: number);
  data: Uint8Array;
  base: number;
  readonly baseAddr: number;
  contains(addr: number): boolean;
  readUint8(addr: number): number;
  readUint16(addr: number): number;
  readUint32(addr: number): number;
  writeUint8(addr: number, value: number): void;
  writeUint16(addr: number, value: number): void;
  writeUint32(addr: number, value: number): void;
  set(src: Uint8Array, offset: number): void;
  copy(src: Memory, srcOffset: number, destOffset: number, length: number): void;
  createView(base: number, offset: number, length: number): Memory;
  remap(base: number): Memory;
}

export class ReadonlyMemory extends Memory {
  static override: boolean;
}

export class MemoryTranslator {
  constructor(base: Memory, delta: number);
  readUint8(addr: number): number;
  readUint16(addr: number): number;
  readUint32(addr: number): number;
  writeUint8(addr: number, value: number): void;
  writeUint16(addr: number, value: number): void;
  writeUint32(addr: number, value: number): void;
}

// ---- GDB ----
export interface GdbTransport {
  onData(cb: (data: string) => void): void;
  write(data: string): void;
}

export class GdbSession {
  constructor(sim: unknown, target: ESP32, transport: GdbTransport, threadCtrl?: unknown);
  updateCurrentThread(coreIdx: number): void;
  onBreak(coreIdx: number): void;
  handleCommand(cmd: string): string;
}

// ---- Enums (values are numeric) ----
export const IOPinState: Readonly<Record<string, number>>;
export const SignalDirection: Readonly<Record<string, number>>;
export const PinPeripheral: Readonly<Record<string, number>>;
