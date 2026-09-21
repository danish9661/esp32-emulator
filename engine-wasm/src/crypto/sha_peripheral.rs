// Faithful 1:1 translation of ShaPeripheral from sha-ecc-key.js
// Handles SHA MMIO at base addr 0x3ff03000 — called directly from WASM memory layer,
// no JS bridge crossing.

use super::sha::ShaEngine;

// Peripheral mode constants (match ShaPeripheralMode enum in JS)
const MODE_SHA1: u32 = 0;
const MODE_SHA256: u32 = 1;
const MODE_SHA384: u32 = 2;
const MODE_SHA512: u32 = 3;

// Operation constants (match ShaOperation enum in JS)
const OP_START: u32 = 0;
const OP_CONTINUE: u32 = 1;
const OP_FINAL: u32 = 2;

// Register offsets (rsaReg7..rsaReg22)
const REG_SHA1_START: u32    = 0x80;
const REG_SHA1_CONTINUE: u32 = 0x84;
const REG_SHA1_FINAL: u32    = 0x88;
const REG_SHA1_BUSY: u32     = 0x8C;
const REG_SHA256_START: u32  = 0x90;
const REG_SHA256_CONTINUE: u32 = 0x94;
const REG_SHA256_FINAL: u32  = 0x98;
const REG_SHA256_BUSY: u32   = 0x9C;
const REG_SHA384_START: u32  = 0xA0;
const REG_SHA384_CONTINUE: u32 = 0xA4;
const REG_SHA384_FINAL: u32  = 0xA8;
const REG_SHA384_BUSY: u32   = 0xAC;
const REG_SHA512_START: u32  = 0xB0;
const REG_SHA512_CONTINUE: u32 = 0xB4;
const REG_SHA512_FINAL: u32  = 0xB8;
const REG_SHA512_BUSY: u32   = 0xBC;

const REG_DATA_END: u32 = 0x80;

// rsaReg23 — expected digest sizes per mode
const DIGEST_SIZE_SHA1: u32   = 20;
const DIGEST_SIZE_SHA256: u32 = 32;
const DIGEST_SIZE_SHA384: u32 = 48;
const DIGEST_SIZE_SHA512: u32 = 64;

fn digest_size_for_mode(mode: u32) -> u32 {
    match mode {
        MODE_SHA1 => DIGEST_SIZE_SHA1,
        MODE_SHA256 => DIGEST_SIZE_SHA256,
        MODE_SHA384 => DIGEST_SIZE_SHA384,
        MODE_SHA512 => DIGEST_SIZE_SHA512,
        _ => 32,
    }
}

fn engine_index_for_mode(mode: u32) -> usize {
    match mode {
        MODE_SHA1 => 0,
        MODE_SHA256 => 1,
        MODE_SHA384 | MODE_SHA512 => 2,
        _ => 1,
    }
}

fn mode_to_algorithm(mode: u32) -> u32 {
    match mode {
        MODE_SHA1 => 0,
        MODE_SHA256 => 2,
        MODE_SHA384 => 3,
        MODE_SHA512 => 4,
        _ => 2,
    }
}

fn busy_engine_mode(mode: u32) -> u32 {
    if mode == MODE_SHA384 { MODE_SHA512 } else { mode }
}

fn busy_check_mode(mode: u32) -> u32 {
    mode
}

pub struct ShaPeripheral {
    sha_buffer: [u8; 128],
    sha_busy: [u32; 4],
    sha_enabled: [u32; 4],
    engines: [ShaEngine; 3],
}

impl ShaPeripheral {
    pub const fn new() -> Self {
        ShaPeripheral {
            sha_buffer: [0u8; 128],
            sha_busy: [0u32; 4],
            sha_enabled: [0u32; 4],
            engines: [
                ShaEngine::new(),
                ShaEngine::new(),
                ShaEngine::new(),
            ],
        }
    }

    pub fn reset(&mut self) {
        self.sha_buffer.fill(0);
        self.sha_busy.fill(0);
        self.sha_enabled.fill(0);
    }

    fn engine_mut(&mut self, mode: u32) -> &mut ShaEngine {
        let idx = engine_index_for_mode(mode);
        &mut self.engines[idx]
    }

    fn sha_complete(&mut self, mode: u32) {
        let bm = busy_engine_mode(mode);
        self.sha_busy[bm as usize] = 0;
    }

    fn sha_write(&mut self, bit0: u32, mode: u32, op: u32) {
        let clock_mode = busy_engine_mode(mode);
        let check_mode = busy_check_mode(mode);
        if bit0 != 0 && self.sha_busy[check_mode as usize] == 0 {
            if op == OP_START {
                self.sha_enabled[clock_mode as usize] = 1;
            }
            if self.sha_enabled[clock_mode as usize] == 0 {
                let sz = digest_size_for_mode(mode) as usize;
                self.sha_buffer[..sz].fill(0);
                return;
            }
            self.sha_busy[clock_mode as usize] = 1;
            let algo = mode_to_algorithm(mode);
            let idx = engine_index_for_mode(mode);
            match op {
                OP_START => {
                    self.engines[idx].initialize(algo);
                    self.engines[idx].update(algo, &self.sha_buffer);
                }
                OP_CONTINUE => self.engines[idx].update(algo, &self.sha_buffer),
                OP_FINAL => self.engines[idx].digest(algo, &mut self.sha_buffer),
                _ => {}
            }
            self.sha_complete(mode);
        }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let offset = addr & 0xFF;
        if offset < REG_DATA_END {
            self.sha_buffer[offset as usize..offset as usize + 4]
                .copy_from_slice(&val.to_be_bytes());
            return;
        }
        let bit0 = val & 1;
        match offset {
            REG_SHA256_START  => {
                self.sha_write(bit0, MODE_SHA256, OP_START);
            }
            REG_SHA256_CONTINUE => {
                self.sha_write(bit0, MODE_SHA256, OP_CONTINUE);
            }
            REG_SHA256_FINAL  => {
                self.sha_write(bit0, MODE_SHA256, OP_FINAL);
            }
            REG_SHA1_START    => self.sha_write(bit0, MODE_SHA1, OP_START),
            REG_SHA1_CONTINUE => self.sha_write(bit0, MODE_SHA1, OP_CONTINUE),
            REG_SHA1_FINAL    => self.sha_write(bit0, MODE_SHA1, OP_FINAL),
            REG_SHA384_START  => self.sha_write(bit0, MODE_SHA384, OP_START),
            REG_SHA384_CONTINUE => self.sha_write(bit0, MODE_SHA384, OP_CONTINUE),
            REG_SHA384_FINAL  => self.sha_write(bit0, MODE_SHA384, OP_FINAL),
            REG_SHA512_START  => self.sha_write(bit0, MODE_SHA512, OP_START),
            REG_SHA512_CONTINUE => self.sha_write(bit0, MODE_SHA512, OP_CONTINUE),
            REG_SHA512_FINAL  => self.sha_write(bit0, MODE_SHA512, OP_FINAL),
            _ => {}
        }
    }

    pub fn read_u32(&mut self, addr: u32) -> u32 {
        let offset = addr & 0xFF;
        if offset < REG_DATA_END {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.sha_buffer[offset as usize..offset as usize + 4]);
            return u32::from_be_bytes(bytes);
        }
        match offset {
            REG_SHA256_BUSY => {
                self.sha_busy[MODE_SHA256 as usize]
            }
            REG_SHA1_BUSY   => self.sha_busy[MODE_SHA1 as usize],
            REG_SHA384_BUSY => self.sha_busy[MODE_SHA384 as usize],
            REG_SHA512_BUSY => self.sha_busy[MODE_SHA512 as usize],
            _ => 0,
        }
    }
}
