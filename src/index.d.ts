// Type definitions for esp32-emu
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
  /** Attach a host I2C slave model at a 7-bit address (Worker split mode). Registered addresses ACK; others NACK. */
  attachI2cSlave(bus: number, addr: number): Promise<void>;
  detachI2cSlave(bus: number, addr: number): Promise<void>;
  /** Pre-stage bytes a future master READ will consume (split-mode onRead). */
  pushI2cTx(bus: number, addr: number, bytes: number[] | Uint8Array): Promise<void>;
  /** Drain bytes the master WROTE since the last drain. */
  popI2cRx(bus: number, addr: number): Promise<number[]>;
  /** Drain staged I2C transaction events since the last poll. */
  pollI2c(bus: number): Promise<Array<{ t: string; addr?: number; read?: boolean; byte?: number; wasRead?: boolean }>>;
  /** Pre-stage MISO bytes for future master transfers (split-mode onTransfer). */
  injectSpiMiso(bus: number, bytes: number[] | Uint8Array): Promise<void>;
  /** Drain captured MOSI bytes since the last drain. */
  pollSpiTx(bus: number): Promise<number[]>;
  /** Fresh core-side GPIO output readback (output value, not input stimulus). */
  getGpioOut(pin: number): Promise<{ level: number; dir: number; pull: number }>;
  /** Snapshot of all 40 pins: {levels, dirs, pulls}. */
  sampleGpioOut(): Promise<{ levels: number[]; dirs: number[]; pulls: number[] }>;
  /** Drain staged GPIO output edges since the last poll: [{pin, level}]. */
  pollGpioChanges(): Promise<Array<{ pin: number; level: number }>>;
  /** Drive a per-pin analog voltage at runtime (host-driven ADC inject). */
  setAnalogInput(pin: number, volts: number): Promise<void>;
  /** LEDC/PWM readback: effective duty, Hz (0 = unconfigured), timer index. */
  getLedc(channel: number): Promise<{ duty: number; freq: number; timer: number }>;
  /** GPIO pin driven by an LEDC channel (-1 = none). */
  getLedcPin(channel: number): Promise<number>;
  /** Drain staged I2S TX DMA words for a controller (guest→host). */
  pollI2sTx(idx?: number): Promise<number[]>;
  /** Arm the I2S TX hook (sticky wiring). */
  armI2sTx(): Promise<void>;
  /** Attach a DS18B20-compatible OneWire slave model to a pin. */
  attachOneWire(pin: number): Promise<void>;
  /** Set the OneWire model's temperature in °C. */
  setOneWireTemp(pin: number, celsius: number): Promise<void>;
  /** Replace the OneWire model's scratchpad (8 bytes → CRC recomputed; 9 used as-is). */
  preloadOneWireScratch(pin: number, bytes: number[] | Uint8Array): Promise<void>;
  /** Drain the OneWire model's decoded master traffic since the last poll. */
  pollOneWire(pin: number): Promise<{ log: Array<{ t: string; byte?: number }>; presence: number }>;
  /** 32-bit native MMIO backdoor write (mirrors readMmio). */
  writeMmio(hid: number, addr: number, value: number): Promise<void>;
}

// ---- Host peripheral taps (esp32-emu.md §1-2) ----
export interface I2cSlaveHandlers {
  onWrite?: (byte: number) => boolean | void;
  onRead?: () => number | void;
  onStop?: (wasRead: boolean) => void;
  onStart?: (addr: number, read: boolean) => void;
}

export class I2cTap {
  constructor(chip: ESP32, bus: number);
  bus: number;
  slaves: Map<number, I2cSlaveHandlers & { readQueue: number[]; writeLog: number[] }>;
  onStart: ((addr: number, read: boolean) => boolean | void) | null;
  onWrite: ((byte: number) => boolean | void) | null;
  onRead: (() => number | void) | null;
  onStop: ((wasRead: boolean) => void) | null;
  events: Array<{ t: string; addr?: number; read?: boolean; byte?: number; wasRead?: boolean }>;
  attachSlave(addr: number, handlers?: I2cSlaveHandlers): unknown;
  detachSlave(addr: number): void;
  clearSlaves(): void;
  preloadRead(addr: number, bytes: number[] | Uint8Array): number;
  drainWriteLog(addr: number): number[];
  pollEvents(): Array<{ t: string; addr?: number; read?: boolean; byte?: number; wasRead?: boolean }>;
}

export class SpiTap {
  constructor(chip: ESP32, bus: number);
  bus: number;
  onTransfer: ((txBytes: number[], recvLen: number) => number[] | void) | null;
  misoQueue: number[];
  mosiLog: number[];
  injectMiso(bytes: number[] | Uint8Array): number;
  drainMosi(maxBytes?: number): number[];
  pollTx(maxBytes?: number): number[];
}

export function dallasCrc8(bytes: number[] | Uint8Array): number;

export class OneWireDevice {
  constructor(pin: number);
  pin: number;
  rom: number[];
  scratchpad: number[];
  log: Array<{ t: string; byte?: number }>;
  presenceCount: number;
  setTemperature(celsius: number): number;
  preloadScratch(bytes: number[] | Uint8Array): void;
  pollLog(): { log: Array<{ t: string; byte?: number }>; presence: number };
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
  /** Host I2C slave taps (bus 0 = I2C0, bus 1 = I2C1). Post-create attachable. */
  i2c0: I2cTap;
  i2c1: I2cTap;
  i2c: I2cTap[];
  /** Host SPI transfer taps (index 0..3 = SPI1, SPI0, SPI2, SPI3; user buses are 2/HSPI and 3/VSPI). */
  spi0: SpiTap;
  spi1: SpiTap;
  spi2: SpiTap;
  spi3: SpiTap;
  spi: SpiTap[];

  loadROM(cpuVal: Uint8Array): void;
  loadWasm(wasmBytes: Uint8Array | ArrayBuffer, enabled?: string): Promise<void>;
  reset(): void;
  step(): void;
  mapAddress(cpuVal: number, coreIdx?: number): Uint8Array | null;
  interrupt(cpuVal: number, tmpVal?: boolean, idxVal?: number): void;
  /** Driven GPIO output level of a pin (0/1). Fresh core-side read. */
  getGpioOut(pin: number): number;
  /** Direction of a pin: 0 = in, 1 = out, 2 = inout. */
  getGpioDir(pin: number): number;
  /** Pull mode of a pin: 0 = none, 1 = up, 2 = down, 3 = both. */
  getGpioPull(pin: number): number;
  /** Snapshot of all 40 pins: {levels, dirs, pulls}. */
  sampleGpioOut(): { levels: number[]; dirs: number[]; pulls: number[] };
  /** Subscribe to GPIO output edges. Returns an unsubscribe function. Listener must not re-enter the engine. */
  onGpioOutChange(cb: (changes: Array<{ pin: number; level: number }>) => void): () => void;
  /** Drain staged output edges, fan out to listeners, return them. */
  pollGpioChanges(): Array<{ pin: number; level: number }>;
  /** Read back a configured per-pin analog voltage (volts). */
  getAnalogInput(pin: number): number | undefined;
  /** Drive a per-pin analog voltage at runtime (host-driven ADC inject). */
  setAnalogInput(pin: number, volts: number): void;
  /** Number of LEDC channels (16: 8 HS + 8 LS). */
  ledcChannelCount(): number;
  /** Effective duty of an LEDC channel (fade-interpolated). */
  getLedcDuty(ch: number): number;
  /** Selected timer index of an LEDC channel. */
  getLedcTimer(ch: number): number;
  /** Output frequency of an LEDC channel's timer, in Hz (0 = unconfigured). */
  getLedcFreq(ch: number): number;
  /** GPIO pin driven by an LEDC channel (-1 = none). */
  getLedcPin(ch: number): number;
  /** Attach a DS18B20-compatible OneWire slave model to a pin. Returns the device. */
  attachOneWire(pin: number, device?: OneWireDevice): OneWireDevice;
  /** Detach a OneWire slave model from a pin. */
  detachOneWire(pin: number): void;
  /** Subscribe to guest→host I2S TX DMA words. Returns an unsubscribe function. */
  onI2sTx(cb: (idx: number, words: Uint32Array) => void): () => void;
  /** Drain staged I2S TX words: pollI2sTx() → both controllers, pollI2sTx(idx) → one. */
  pollI2sTx(idx?: number | null): Array<{ idx: number; words: number[] }>;
  /** OneWire slave devices by pin. */
  onewire: Map<number, OneWireDevice>;
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
