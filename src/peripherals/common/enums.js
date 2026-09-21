// Bidirectional enum helper: keeps both `Name: value` and `value: "Name"`
// lookups (the original obfuscated IIFE pattern did the same via assignment).
function defineEnum(map) {
  for (const [key, value] of Object.entries(map)) map[value] = key;
  return map;
}

export const TimerMode = defineEnum({
  Increment: 0,
  Decrement: 1,
  ZigZag: 2,
});

export const PeripheralType = defineEnum({
  GPIO: 0,
  SPI: 1,
  I2C: 2,
  UART: 3,
  LEDC: 4,
  PCNT: 5,
  TWAI: 6,
  Other: 7,
  None: 8,
});

export const PinState = defineEnum({
  Low: 0,
  High: 1,
  Input: 2,
  PullUp: 3,
  PullDown: 4,
});

export const SignalDirection = defineEnum({
  Input: 1,
  Output: 2,
  Both: 3,
});

export const InterruptTrigger = defineEnum({
  Disable: 0,
  RisingEdge: 1,
  FallingEdge: 2,
  Edge: 3,
  LowLevel: 4,
  HighLevel: 5,
});

export const I2cCommand = defineEnum({
  RSTART: 0,
  WRITE: 1,
  READ: 2,
  STOP: 3,
  END: 4,
  RSTART_NEW: 6,
  STOP_NEW: 2,
  READ_NEW: 3,
});

export const I2cInterruptType = defineEnum({
  RXFIFO_FULL: 0,
  TXFIFO_EMPTY: 1,
  RXFIFO_OVF: 2,
  END_DETECT: 3,
  SLAVE_TRAN_COMP: 4,
  ARBITRATION_LOST: 5,
  MASTER_TRAN_COMP: 6,
  TRANS_COMPLETE: 7,
  TIME_OUT: 8,
  TRANS_START: 9,
  ACK_ERR: 10,
  RX_REC_FULL: 11,
  TX_SEND_EMPTY: 12,
});

export const PcntRegister = defineEnum({
  CONF0: 0,
  CONF1: 1,
  CONF2: 2,
  CNT: 3,
  STATUS: 4,
});

export const RmtClockSource = defineEnum({
  APB: 1,
  RC_FAST: 2,
  XTAL: 3,
});

export const RmtChannelRegister = defineEnum({
  CONF0: 0,
  CONF1: 1,
  TX_LIM: 2,
});

export const ShaAlgorithm = defineEnum({
  SHA1: 0,
  SHA224: 1,
  SHA256: 2,
  SHA384: 3,
  SHA512: 4,
  SHA512_224: 5,
  SHA512_256: 6,
});

export const ShaPeripheralMode = defineEnum({
  SHA1: 0,
  SHA256: 1,
  SHA384: 2,
  SHA512: 3,
});

export const ShaOperation = defineEnum({
  Start: 0,
  Continue: 1,
  Final: 2,
});

export const KeyManagerCryptoAlgo = defineEnum({
  AES: 1,
  ECDH0: 2,
  ECDH1: 3,
});

export const KeyPurpose = defineEnum({
  INVALID: 0,
  ECDSA_192: 1,
  ECDSA_256: 2,
  FLASH_256_1: 3,
  FLASH_256_2: 4,
  FLASH_128: 5,
  HMAC: 6,
  DS: 7,
  PSRAM_256_1: 8,
  PSRAM_256_2: 9,
  PSRAM_128: 10,
  ECDSA_384_L: 11,
  ECDSA_384_H: 12,
});

export const KeyManagerState = defineEnum({
  IDLE: 0,
  LOAD: 1,
  GAIN: 2,
  BUSY: 3,
});

export const XtsState = defineEnum({
  IDLE: 0,
  BUSY: 1,
  DONE: 2,
  VISIBLE: 3,
});

export const OutputSignalIndex = defineEnum({
  LEDC_HS_SIG_OUT0: 71,
  LEDC_LS_SIG_OUT0: 79,
  RMT_SIG_OUT0: 87,
});
