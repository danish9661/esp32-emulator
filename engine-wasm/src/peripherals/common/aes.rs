use crate::crypto::aes::*;
use crate::peripherals::common::peripheral::PeripheralBase;
use crate::peripherals::types::*;

const AES_KEY_SIZES: [u32; 4] = [128, 192, 256, 128];
const AES_REG_START: u32 = 0;
const AES_REG_READY: u32 = 4;
const AES_REG_MODE: u32 = 8;
const AES_KEY0_REG: u32 = 16;
const AES_KEY1_REG: u32 = 20;
const AES_KEY2_REG: u32 = 24;
const AES_KEY3_REG: u32 = 28;
const AES_KEY4_REG: u32 = 32;
const AES_KEY5_REG: u32 = 36;
const AES_KEY6_REG: u32 = 40;
const AES_KEY7_REG: u32 = 44;
const AES_TEXT0_REG: u32 = 48;
const AES_TEXT1_REG: u32 = 52;
const AES_TEXT2_REG: u32 = 56;
const AES_TEXT3_REG: u32 = 60;
const AES_REG_ENDIAN: u32 = 64;
const AES_MODE_DECRYPT_BIT: u32 = 4;

pub struct AesPeripheral {
    base: PeripheralBase,
    aes_key: [u32; 8],
    aes_text: [u32; 4],
    key_updated: bool,
    aes_ready: bool,
    aes_endian: u32,
    aes_mode: u32,
    aes_cipher: Option<AesCore>,
}

impl AesPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        AesPeripheral {
            base: PeripheralBase::new(base_addr, name),
            aes_key: [0u32; 8],
            aes_text: [0u32; 4],
            key_updated: false,
            aes_ready: false,
            aes_endian: 0,
            aes_mode: 0,
            aes_cipher: None,
        }
    }

    fn encrypt_complete(&mut self) {
        self.aes_ready = true;
    }

    fn on_key_updated(&mut self) {
        self.aes_cipher = None;
    }

    fn on_encrypt(&mut self, decrypt: u32, key_bits: u32) {
        let key_bytes = (key_bits >> 3) as usize;
        let mut reg_val = [0u8; 32];
        for i in 0..key_bytes {
            reg_val[i] = (self.aes_key[i >> 2] >> ((i & 3) << 3)) as u8;
        }
        if self.aes_cipher.is_none() {
            self.aes_cipher = Some(AesCore::new(&reg_val[..key_bytes]));
        }
        let mut arg_val = [0u8; 16];
        for i in 0..16 {
            arg_val[i] = (self.aes_text[i >> 2] >> ((i & 3) << 3)) as u8;
        }
        let result = if decrypt != 0 {
            self.aes_cipher.as_ref().unwrap().decrypt(&arg_val)
        } else {
            self.aes_cipher.as_ref().unwrap().encrypt(&arg_val)
        };
        for i in 0..4 {
            self.aes_text[i] = result[4 * i] as u32
                | (result[4 * i + 1] as u32) << 8
                | (result[4 * i + 2] as u32) << 16
                | (result[4 * i + 3] as u32) << 24;
        }
        self.encrypt_complete();
    }

    fn start(&mut self) {
        let cpu_val = AES_KEY_SIZES[(3 & self.aes_mode) as usize];
        let tmp_val = self.aes_mode & AES_MODE_DECRYPT_BIT;
        if self.key_updated {
            self.on_key_updated();
            self.key_updated = false;
        }
        self.aes_ready = false;
        self.on_encrypt(tmp_val, cpu_val);
    }
}

impl MmioPeripheral for AesPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr & 0xFFF;
        match offset {
            AES_REG_READY => self.aes_ready as u32,
            AES_KEY0_REG | AES_KEY1_REG | AES_KEY2_REG | AES_KEY3_REG
            | AES_KEY4_REG | AES_KEY5_REG | AES_KEY6_REG | AES_KEY7_REG => {
                let idx = (offset - AES_KEY0_REG) >> 2;
                self.aes_key[idx as usize]
            }
            AES_TEXT0_REG | AES_TEXT1_REG | AES_TEXT2_REG | AES_TEXT3_REG => {
                let idx = (offset - AES_TEXT0_REG) >> 2;
                self.aes_text[idx as usize]
            }
            AES_REG_MODE => self.aes_mode,
            AES_REG_ENDIAN => self.aes_endian,
            _ => self.base.read_uint32(addr),
        }
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr & 0xFFF;
        match offset {
            AES_REG_START => {
                if (1 & val) != 0 {
                    self.start();
                }
            }
            AES_REG_MODE => {
                self.aes_mode = val;
            }
            AES_KEY0_REG | AES_KEY1_REG | AES_KEY2_REG | AES_KEY3_REG
            | AES_KEY4_REG | AES_KEY5_REG | AES_KEY6_REG | AES_KEY7_REG => {
                let idx = (offset - AES_KEY0_REG) >> 2;
                self.aes_key[idx as usize] = val;
                self.key_updated = true;
            }
            AES_TEXT0_REG | AES_TEXT1_REG | AES_TEXT2_REG | AES_TEXT3_REG => {
                let idx = (offset - AES_TEXT0_REG) >> 2;
                self.aes_text[idx as usize] = val;
            }
            AES_REG_ENDIAN => {
                self.aes_endian = val;
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.aes_ready = true;
        self.aes_mode = 0;
        self.aes_endian = 63;
        self.aes_key = [0u32; 8];
        self.aes_text = [0u32; 4];
        self.base.memory.fill(0);
    }
}

pub use crate::crypto::aes::{
    AesCbc, AesCfb128, AesCfb8, AesCore, AesCtr, AesEcb, AesGcm, AesOfb,
};
