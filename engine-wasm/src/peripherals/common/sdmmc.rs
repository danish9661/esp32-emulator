use crate::peripherals::common::peripheral::PeripheralBase;
use crate::peripherals::types::*;

const FLASH_REG1: u32 = 64;
const REG_FN: u32 = 0;
const FLASH_REG2: u32 = 20;
const FLASH_REG3: u32 = 28;
const FLASH_REG4: u32 = 32;
const FLASH_REG5: u32 = 36;
const FLASH_REG6: u32 = 40;
const FLASH_REG7: u32 = 44;
const FLASH_REG8: u32 = 48;
const FLASH_REG9: u32 = 52;
const FLASH_REG10: u32 = 56;
const FLASH_REG11: u32 = 60;
const FLASH_REG12: u32 = 64;
const FLASH_REG13: u32 = 68;
const FLASH_REG14: u32 = 72;
const FLASH_REG15: u32 = 76;
const FLASH_REG16: u32 = 80;
const FLASH_REG17: u32 = 84;
const FLASH_REG18: u32 = 100;
const FLASH_REG19: u32 = 108;
const FLASH_REG20: u32 = 112;
const FLASH_REG21: u32 = 128;
const FLASH_REG22: u32 = 132;
const FLASH_REG23: u32 = 136;
const FLASH_REG24: u32 = 140;
const FLASH_REG25: u32 = 144;
const FLASH_REG26: u32 = 148;
const FLASH_REG27: u32 = 512;
const FLASH_REG28: u32 = 0x80000000;
const FLASH_REG29: u32 = 2097152;
const FLASH_REG30: u32 = 1024;
const FLASH_REG31: u32 = 512;
const FLASH_REG32: u32 = 64;
const FLASH_REG33: u32 = 63;
const FLASH_REG34: u32 = 1;
const FLASH_REG35: u32 = 2;
const FLASH_REG36: u32 = 4;
const FLASH_REG37: u32 = 16;
const FLASH_REG38: u32 = 0x2000000;
const FLASH_REG39: u32 = 4;
const FLASH_REG40: u32 = 8;
const FLASH_REG41: u32 = 256;
const FLASH_REG42: u32 = 17;
const FLASH_REG43: u32 = 8191;
const FLASH_REG44: u32 = 1;
const FLASH_REG45: u32 = 4;
const FLASH_REG46: u32 = 8;
const FLASH_REG47: u32 = 16;
const FLASH_REG48: u32 = 32;
const FLASH_REG49: u32 = 256;
const FLASH_REG50: u32 = 1;
const FLASH_REG51: u32 = 128;
const FLASH_REG52: u32 = 1;
const FLASH_REG53: u32 = 2;
const FLASH_REG54: u32 = 16;
const FLASH_REG55: u32 = 256;
const FLASH_REG56: u32 = 512;
const FLASH_REG57: u32 = 0x80000000;
const FLASH_REG58: u32 = 32;
const FLASH_REG59: u32 = 16;
const FLASH_REG60: u32 = 4;
const RSA_REG1: u32 = 0x44cc345;
const RSA_REG2: u32 = 64;

pub struct CmdResult {
    pub long: bool,
    pub data: [u32; 4],
}

pub type CommandCallback = fn(u32, u32, bool) -> Option<CmdResult>;
pub type ReadDataCallback = fn(u32, u32, &mut [u8]) -> u32;
pub type WriteDataCallback = fn(u32, &[u8]);

pub struct SdmmcConfig {
    pub cd_signals: [u32; 2],
    pub cd_count: u32,
    pub wp_signals: [u32; 2],
    pub wp_count: u32,
    pub irq: u32,
}

trait CoreMemoryAccess {
    fn read_memory_u32(&self, addr: u32) -> u32;
    fn read_memory_u8(&self, addr: u32) -> u8;
    fn write_memory_u32(&mut self, addr: u32, val: u32);
    fn write_memory_u8(&mut self, addr: u32, val: u8);
}

impl CoreMemoryAccess for CpuContext<'_> {
    fn read_memory_u32(&self, addr: u32) -> u32 {
        crate::xtensa::memory::dma_read_u32(addr)
    }
    fn read_memory_u8(&self, addr: u32) -> u8 {
        crate::xtensa::memory::dma_read_u8(addr)
    }
    fn write_memory_u32(&mut self, addr: u32, val: u32) {
        crate::xtensa::memory::dma_write_u32(addr, val);
    }
    fn write_memory_u8(&mut self, addr: u32, val: u8) {
        crate::xtensa::memory::dma_write_u8(addr, val as u32);
    }
}

pub struct SdioSlavePeripheral {
    pub base: PeripheralBase,
}

impl SdioSlavePeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        SdioSlavePeripheral {
            base: PeripheralBase::new(base_addr, name),
        }
    }
}

impl MmioPeripheral for SdioSlavePeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        // NOTE: offsets via mask, not subtraction — the region writer may
        // pass a page-stripped address (addr & 0xFFC); bases are 4KB-aligned
        // so this is identical for well-formed addresses and robust
        // otherwise (a subtraction-based offset silently misroutes those).
        if (addr & 0xFFF) == FLASH_REG1 {
            0xffffffff
        } else {
            self.base.read_uint32(addr)
        }
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr & 0xFFF;
        self.base.write_uint32(addr, val);
        // RX reset completion: the driver resets RX (CONF0 RX_RST) and then
        // spins on INT_RAW RX_DONE (e.g. in send_start with a loaded RX
        // buffer). Real HW raises DONE when the reset completes; without it
        // sdio_slave_start() hangs forever. Data-path DONE bits (RX_DONE on
        // actual reception, etc.) stay clear — with no host nothing arrives,
        // so recv() still correctly times out.
        if offset == 0 && (val & 0x2) != 0 {
            let cur = self.base.read_uint32(self.base.base_addr + 4);
            self.base
                .write_uint32(self.base.base_addr + 4, cur | 0x1_0000);
        }
    }

    fn reset(&mut self) {
        self.base.reset();
    }
}

pub struct SdmmcPeripheral {
    pub base: PeripheralBase,
    pub config: SdmmcConfig,
    pub slot_cd_state: [bool; 2],
    pub slot_wp_state: [bool; 2],
    pub external_card_present: bool,
    pub app_cmd_pending: bool,
    pub data_buffer: [u8; 4096],
    pub data_offset: u32,
    pub data_length: u32,
    pub is_write_operation: bool,
    pub current_block_number: u32,
    pub blocks_remaining: u32,
    pub fifo: [u32; 64],
    pub fifo_read_index: u32,
    pub fifo_write_index: u32,
    pub fifo_count: u32,
    pub on_command: Option<CommandCallback>,
    pub on_read_data: Option<ReadDataCallback>,
    pub on_write_data: Option<WriteDataCallback>,
    pub pending_cmd_interrupts: u32,
    // ---- Virtual SD card state (internal card, no host callbacks needed) ----
    // SD mode card state machine: 0=idle, 1=ready, 2=ident, 3=stby, 4=tran.
    pub sd_state: u32,
    pub sd_rca: u32,
    pub sd_selected: bool,
    pub sd_blocklen: u32,
    pub sd_bus_width_4: bool,
    // eMMC mode (config.sdCard.type == "mmc"): the card answers the MMC
    // probe path (CMD1 OCR busy-then-ready, host-assigned RCA, MMC
    // CID/CSD, CMD8 SEND_EXT_CSD + CMD6 SWITCH) while the SD probe
    // (CMD8/0x1AA, CMD55) gets no response so it times out and the
    // driver falls back to MMC. Persists across reset (card config).
    pub mmc_mode: bool,
    pub mmc_ocr_polls: u32,
    pub ext_csd_pending: bool,
    pub mmc_bus_width: u32,
}

impl SdmmcPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: SdmmcConfig) -> Self {
        SdmmcPeripheral {
            base: PeripheralBase::new(base_addr, name),
            config,
            slot_cd_state: [true, true],
            slot_wp_state: [false, false],
            external_card_present: false,
            app_cmd_pending: false,
            data_buffer: [0u8; 4096],
            data_offset: 0,
            data_length: 0,
            is_write_operation: false,
            current_block_number: 0,
            blocks_remaining: 0,
            fifo: [0u32; 64],
            fifo_read_index: 0,
            fifo_write_index: 0,
            fifo_count: 0,
            on_command: None,
            on_read_data: None,
            on_write_data: None,
            pending_cmd_interrupts: 0,
            sd_state: 0,
            sd_rca: 0,
            sd_selected: false,
            sd_blocklen: 512,
            sd_bus_width_4: false,
            mmc_mode: false,
            mmc_ocr_polls: 0,
            ext_csd_pending: false,
            mmc_bus_width: 0,
        }
    }

    // ---- Virtual SD card: block storage via the JS host (chip.sdData) ----
    fn sd_num_blocks() -> u32 {
        let n = unsafe { crate::peripherals::common::ffi::js_sd_num_blocks() };
        if n == 0 { 32768 } else { n }
    }

    fn sd_read_block_internal(block: u32, buf: &mut [u8; 512]) {
        let n = Self::sd_num_blocks();
        if block >= n {
            buf.fill(0);
            return;
        }
        unsafe {
            crate::peripherals::common::ffi::js_sd_read_block(block, buf.as_mut_ptr() as u32);
        }
    }

    fn sd_write_block_internal(block: u32, buf: &[u8; 512]) {
        let n = Self::sd_num_blocks();
        if block >= n {
            return;
        }
        unsafe {
            crate::peripherals::common::ffi::js_sd_write_block(block, buf.as_ptr() as u32);
        }
    }

    // CSD v2.0 (SDHC/SDXC) for the current virtual capacity.
    // Capacity = (C_SIZE+1) * 512KB; C_SIZE = num_blocks/1024 - 1.
    // Word order: RESP0 = CSD[31:0] ... RESP3 = CSD[127:96] (driver
    // MMC_RSP_BITS indexes response[0] as bits 31:0 — verified: TRAN_SPEED
    // 0x32 and CCC 0x5B5 decode from these positions).
    fn sd_csd() -> [u32; 4] {
        let blocks = Self::sd_num_blocks();
        let c_size: u32 = blocks.saturating_sub(1024) / 1024;
        let r3: u32 = 0x400E0032; // CSD_STRUCTURE=01, TAAC=0x0E, TRAN_SPEED=0x32 (25MHz)
        // CCC=0x5B5 (95:84), READ_BL_LEN=9 (83:80), C_SIZE high 6 (69:64)
        let r2: u32 = (0x5B5 << 20) | (9 << 16) | ((c_size >> 16) & 0x3F);
        // C_SIZE low 16 (63:48), ERASE_BLK_EN (46), SECTOR_SIZE=0x7F (45:39)
        let r1: u32 = ((c_size & 0xFFFF) << 16) | (1 << 14) | (0x7F << 7);
        // R2W_FACTOR=2 (28:26), WRITE_BL_LEN=9 (25:22), end bit
        let r0: u32 = (2 << 26) | (9 << 22) | 1;
        [r0, r1, r2, r3]
    }

    // Fixed CID (MID=0x03, OID="SD", PNM="EMUSP", PRV=1.0).
    fn sd_cid() -> [u32; 4] {
        [0x78192301, 0x10123456, 0x4D555350, 0x03534445]
    }

    // Fixed MMC CID (MID=0x15, CBX=1, OID=0x0100, PNM="EMMC01", PRV=1.0,
    // PSN=0x12345678). Same RESP word order as sd_cid.
    fn mmc_cid() -> [u32; 4] {
        [0x3456781A, 0x30311012, 0x454D4D43, 0x15010100]
    }

    // MMC CSD (v1.x layout) for the current virtual capacity. TRAN_SPEED
    // 0x32 (26MHz) lives in r3[7:0] (CSD[103:96]); CCC=0x5B5,
    // READ_BL_LEN=9, C_SIZE=0xFFF/C_SIZE_MULT=7 (nonzero placeholder —
    // sector-mode capacity comes from EXT_CSD SEC_COUNT, OCR bit30).
    // Same RESP word order as sd_csd.
    fn mmc_csd() -> [u32; 4] {
        let r3: u32 = (1 << 30) | (0x0E << 16) | 0x32;
        let r2: u32 = (0x5B5 << 20) | (9 << 16) | 0x3FF;
        let r1: u32 = 0xC0000000 | (7 << 15) | (1 << 14) | (0x7F << 7);
        let r0: u32 = (2 << 26) | (9 << 22) | 1;
        [r0, r1, r2, r3]
    }

    // 512B EXT_CSD: REV=8 (v1.8), CARD_TYPE=0x01 (26MHz only, no HS
    // switch), BUS_WIDTH/HS_TIMING=0 (legacy 1-bit), SEC_COUNT [212:215]
    // = virtual block count (sector-mode capacity, OCR bit30).
    fn mmc_ext_csd() -> [u8; 512] {
        let mut b = [0u8; 512];
        b[192] = 8;
        b[196] = 0x01;
        b[212..216].copy_from_slice(&Self::sd_num_blocks().to_le_bytes());
        b
    }

    // EXT_CSD delivery marker (distinct from the ACMD51 SCR marker).
    const EXT_CSD_MARKER: u32 = 0xFFFF_FFFE;

    // Virtual-card command handler. Returns:
    //   Some((resp_words, is_long)) for commands with a response,
    //   None for commands with no response (CMD0).
    // Data-transfer commands also arm blocks_remaining/current_block_number;
    // the shared start_data_transfer path then moves the blocks.
    fn sd_handle_command(&mut self, cmd_idx: u32, arg: u32) -> Option<([u32; 4], bool)> {
        const R1_READY: u32 = 0x100; // READY_FOR_DATA, no error bits
        match cmd_idx {
            0 => {
                // GO_IDLE_STATE: back to idle, no response.
                self.sd_state = 0;
                self.sd_rca = 0;
                self.sd_selected = false;
                self.app_cmd_pending = false;
                self.mmc_ocr_polls = 0;
                None
            }
            1 => {
                // MMC SEND_OP_COND (R3 OCR, no ACMD55): busy for the first
                // polls so the driver's retry loop is exercised, then
                // ready + sector mode (bits 31+30). SD mode: bare CMD1 is
                // meaningless -> default R1 below.
                if self.mmc_mode {
                    if self.mmc_ocr_polls < 2 {
                        self.mmc_ocr_polls += 1;
                        Some(([0x00FF8000, 0, 0, 0], false))
                    } else {
                        self.sd_state = 1; // ready
                        Some(([0xC0FF8000, 0, 0, 0], false))
                    }
                } else {
                    self.app_cmd_pending = false;
                    Some(([R1_READY, 0, 0, 0], false))
                }
            }
            8 => {
                if arg == 0x1AA {
                    // SD SEND_IF_COND: echo voltage + check pattern (R7).
                    // MMC mode never reaches here (starved to timeout in
                    // start_command so the SD probe fails).
                    Some(([arg & 0xFFF, 0, 0, 0], false))
                } else if arg == 0 && self.mmc_mode {
                    // MMC SEND_EXT_CSD: R1 + 512B EXT_CSD via the data path
                    // (driver sends it with data-expected set).
                    let ext = Self::mmc_ext_csd();
                    self.data_buffer[..512].copy_from_slice(&ext);
                    self.data_offset = 0;
                    self.data_length = 512;
                    self.is_write_operation = false;
                    self.blocks_remaining = 1;
                    self.ext_csd_pending = true;
                    self.current_block_number = Self::EXT_CSD_MARKER;
                    Some(([R1_READY, 0, 0, 0], false))
                } else {
                    Some(([R1_READY, 0, 0, 0], false))
                }
            }
            55 => {
                // APP_CMD: next command is application-specific (R1).
                self.app_cmd_pending = true;
                Some(([R1_READY | 0x20, 0, 0, 0], false))
            }
            41 if self.app_cmd_pending => {
                // SD_SEND_OP_COND (R3 = OCR: power-up done + HCS + 2.7-3.6V).
                self.app_cmd_pending = false;
                self.sd_state = 1; // ready
                Some(([0xC0FF8000, 0, 0, 0], false))
            }
            2 => {
                // ALL_SEND_CID (R2, 136-bit).
                self.sd_state = 2; // ident
                let cid = if self.mmc_mode { Self::mmc_cid() } else { Self::sd_cid() };
                Some((cid, true))
            }
            3 => {
                // SEND_RELATIVE_ADDR: SD (arg RCA 0) -> card assigns;
                // MMC -> host assigns the RCA in the argument.
                if (arg >> 16) != 0 {
                    self.sd_rca = arg >> 16;
                    self.sd_state = 3; // stby
                    Some(([(self.sd_rca << 16) | R1_READY, 0, 0, 0], false))
                } else {
                    self.sd_rca = 0x1234;
                    self.sd_state = 3; // stby
                    Some(([self.sd_rca << 16, 0, 0, 0], false))
                }
            }
            7 => {
                // SELECT_CARD: RCA match -> tran, else stby (R1).
                if (arg >> 16) == self.sd_rca {
                    self.sd_selected = true;
                    self.sd_state = 4; // tran
                } else {
                    self.sd_selected = false;
                    self.sd_state = 3; // stby
                }
                Some(([R1_READY, 0, 0, 0], false))
            }
            9 => {
                // SEND_CSD, addressed by RCA (R2).
                let csd = if self.mmc_mode { Self::mmc_csd() } else { Self::sd_csd() };
                Some((csd, true))
            }
            10 => {
                // SEND_CID, addressed by RCA (R2).
                let cid = if self.mmc_mode { Self::mmc_cid() } else { Self::sd_cid() };
                Some((cid, true))
            }
            12 => {
                // STOP_TRANSMISSION (R1).
                self.blocks_remaining = 0;
                Some(([R1_READY, 0, 0, 0], false))
            }
            13 => {
                // SEND_STATUS (R1: READY + current state in bits 12:9).
                Some(([R1_READY | (self.sd_state << 9), 0, 0, 0], false))
            }
            16 => {
                // SET_BLOCKLEN (R1). SDHC ignores it (fixed 512); accept any.
                self.sd_blocklen = arg;
                Some(([R1_READY, 0, 0, 0], false))
            }
            6 if self.app_cmd_pending => {
                // ACMD6 SET_BUS_WIDTH (R1): bit1 = 4-bit mode.
                self.app_cmd_pending = false;
                self.sd_bus_width_4 = (arg & 2) != 0;
                Some(([R1_READY, 0, 0, 0], false))
            }
            6 => {
                // MMC SWITCH (no ACMD55 in MMC mode): Access=(arg>>24)&3,
                // Index=(arg>>16)&0xFF, Value=(arg>>8)&0xFF. Index 183
                // (BUS_WIDTH) is recorded; everything completes instantly
                // (R1 ready, DAT0 never goes busy). SD bare CMD6 keeps the
                // old default-R1 behavior.
                if self.mmc_mode {
                    let index = (arg >> 16) & 0xFF;
                    let value = (arg >> 8) & 0xFF;
                    if index == 183 {
                        self.mmc_bus_width = value;
                    }
                }
                Some(([R1_READY, 0, 0, 0], false))
            }
            51 if self.app_cmd_pending => {
                // ACMD51 SEND_SCR: 8-byte SCR via data path (R1 + read).
                // SCR[0]: SD_SPEC=2 (v2.00), SD_BUS_WIDTHS bit2 (4-bit OK).
                self.app_cmd_pending = false;
                self.data_buffer[0] = 0x02;
                self.data_buffer[1] = 0x25;
                for i in 2..8 {
                    self.data_buffer[i] = 0;
                }
                self.data_offset = 0;
                self.data_length = 8;
                self.is_write_operation = false;
                self.blocks_remaining = 1;
                self.current_block_number = 0xFFFF_FFFF; // SCR marker (not a real block)
                Some(([R1_READY, 0, 0, 0], false))
            }
            _ => {
                // Default: R1 OK. Data commands (17/18/24/25) arm the transfer
                // here; start_command routes them into start_data_transfer.
                self.app_cmd_pending = false;
                if cmd_idx == 17 || cmd_idx == 18 {
                    // READ_SINGLE/MULTIPLE_BLOCK: SDHC = block addressing.
                    self.is_write_operation = false;
                    self.current_block_number = arg;
                    Some(([R1_READY, 0, 0, 0], false))
                } else if cmd_idx == 24 || cmd_idx == 25 {
                    // WRITE_BLOCK/MULTIPLE_BLOCK.
                    self.is_write_operation = true;
                    self.current_block_number = arg;
                    Some(([R1_READY, 0, 0, 0], false))
                } else {
                    Some(([R1_READY, 0, 0, 0], false))
                }
            }
        }
    }

    fn card_present(&self) -> bool {
        self.external_card_present || self.slot_cd_state.iter().any(|&v| v)
    }

    pub fn on_cmd_complete(&mut self, ctx: &mut CpuContext) {
        if self.pending_cmd_interrupts != 0 {
            self.base.set_register_bits(FLASH_REG13, self.pending_cmd_interrupts);
            self.pending_cmd_interrupts = 0;
            self.update_interrupt(ctx);
        }
    }

    fn schedule_command_complete(&mut self, ctx: &mut CpuContext, val: u32) {
        self.pending_cmd_interrupts |= val;
        // Synchronous delivery (matches every other peripheral's *_complete
        // path in this engine: set the status bits + raise the IRQ inline).
        // The old code scheduled an 80-tick clock event per command; under
        // the MMC init storm (CMD1 retries + CID/CSD + EXT_CSD back-to-back)
        // the queue backlog delayed delivery past the driver's
        // wait_for_event timeout (0x107), even though the payload landed.
        // The driver polls the status register in a tight loop, so inline
        // delivery is invisible timing-wise and cannot backlog.
        self.on_cmd_complete(ctx);
    }

    fn set_card_present(&mut self, ctx: &mut CpuContext, val: bool) {
        self.external_card_present = val;
        self.update_cdetect();
        self.base.set_register_bits(FLASH_REG13, FLASH_REG44);
        self.update_interrupt(ctx);
    }

    fn set_write_protected(&mut self, val: bool) {
        for t in 0..self.slot_wp_state.len() {
            self.slot_wp_state[t] = val;
        }
        self.update_wrtprt();
    }

    fn set_sdio_interrupt(&mut self, ctx: &mut CpuContext, val: u32) {
        if val < 16 {
            self.base.set_register_bits(FLASH_REG13, 1 << (16 + val));
            self.update_interrupt(ctx);
        }
    }

    fn update_cdetect(&mut self) {
        let mut val = 0u32;
        for t in 0..self.slot_cd_state.len() {
            if !self.slot_cd_state[t] && !self.external_card_present {
                val |= 1 << t;
            }
        }
        self.base.write_register(FLASH_REG16, val);
    }

    fn update_wrtprt(&mut self) {
        let mut val = 0u32;
        for t in 0..self.slot_wp_state.len() {
            if self.slot_wp_state[t] {
                val |= 1 << t;
            }
        }
        self.base.write_register(FLASH_REG17, val);
    }

    fn update_status(&mut self) {
        let mut val = 0u32;
        if self.fifo_count == 0 {
            val |= FLASH_REG39;
        }
        if self.fifo_count == RSA_REG2 {
            val |= FLASH_REG40;
        }
        val |= (self.fifo_count & FLASH_REG43) << FLASH_REG42;
        if self.card_present() {
            val |= FLASH_REG41;
        }
        self.base.write_register(FLASH_REG14, val);
    }

    fn update_interrupt(&mut self, ctx: &mut CpuContext) {
        let raw = self.base.read_register(FLASH_REG13);
        let mask = self.base.read_register(FLASH_REG5);
        let ctrl = self.base.read_register(REG_FN);
        let clr_en = self.base.read_register(FLASH_REG24);
        let set_en = self.base.read_register(FLASH_REG25);
        let reg_val = raw & mask;
        self.base.write_register(FLASH_REG12, reg_val);
        let arg_val = (clr_en & set_en) != 0;
        let register_type = (ctrl & FLASH_REG37) != 0 && (reg_val != 0 || arg_val);
        ctx.interrupt(self.config.irq, register_type);
    }

    fn handle_ctrl_write(&mut self, ctx: &mut CpuContext, mut val: u32) {
        if (val & FLASH_REG34) != 0 {
            self.reset();
            val &= !FLASH_REG34;
        }
        if (val & FLASH_REG35) != 0 {
            self.fifo_read_index = 0;
            self.fifo_write_index = 0;
            self.fifo_count = 0;
            self.update_status();
            val &= !FLASH_REG35;
        }
        if (val & FLASH_REG36) != 0 {
            self.base.write_register(FLASH_REG24, 0);
            val &= !FLASH_REG36;
        }
        self.base.write_register(REG_FN, val);
        self.update_interrupt(ctx);
    }

    fn handle_bmod_write(&mut self, mut val: u32) {
        if (val & FLASH_REG50) != 0 {
            self.base.write_register(FLASH_REG24, 0);
            val &= !FLASH_REG50;
        }
        self.base.write_register(FLASH_REG21, val);
    }

    fn start_command(&mut self, ctx: &mut CpuContext, val: u32) {
        let cmd_idx = val & FLASH_REG33;
        let arg = self.base.read_register(FLASH_REG6);
        if (val & FLASH_REG29) != 0 {
            self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
            return;
        }
        let on_cmd = self.on_command;
        if let Some(cmd_fn) = on_cmd {
            let result = cmd_fn(cmd_idx, arg, self.app_cmd_pending);
            self.app_cmd_pending = cmd_idx == 55;
            if let Some(cmd_result) = result {
                if cmd_result.long {
                    self.base.write_register(FLASH_REG8, cmd_result.data[0]);
                    self.base.write_register(FLASH_REG9, cmd_result.data[1]);
                    self.base.write_register(FLASH_REG10, cmd_result.data[2]);
                    self.base.write_register(FLASH_REG11, cmd_result.data[3]);
                } else {
                    self.base.write_register(FLASH_REG8, cmd_result.data[0]);
                }
            } else if (val & FLASH_REG32) != 0 {
                self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
                self.schedule_command_complete(ctx, FLASH_REG49 | FLASH_REG45);
                return;
            }
            self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
            if (val & FLASH_REG31) != 0 {
                self.start_data_transfer(ctx, val);
            } else {
                self.schedule_command_complete(ctx, FLASH_REG45);
            }
            return;
        }
        // No host callback: use the internal virtual SD card.
        // CMD5 (IO_SEND_OP_COND) is special: SD memory cards never respond
        // (R4 timeout). IDF sdmmc_init_io treats the timeout as
        // "memory-only card" (is_mem=1); any response at all is decoded as an
        // SDIO card and the whole SD stack is skipped. So: timeout, no RESP.
        if cmd_idx == 5 {
            if crate::native_mmio::sd_trace_active() {
                crate::native_mmio::sd_log_cmd(cmd_idx, arg, 0xFFFF_FFFF, 0);
            }
            self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
            self.schedule_command_complete(ctx, FLASH_REG49 | FLASH_REG45);
            return;
        }
        // eMMC mode: the SD probe (CMD8 SEND_IF_COND, CMD55 APP_CMD) gets
        // no response so it times out and the driver falls back to the MMC
        // path (CMD1...). Mirrors the CMD5 timeout above.
        if self.mmc_mode && (cmd_idx == 55 || (cmd_idx == 8 && arg == 0x1AA)) {
            if crate::native_mmio::sd_trace_active() {
                crate::native_mmio::sd_log_cmd(cmd_idx, arg, 0xFFFF_FFFF, 0);
            }
            self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
            self.schedule_command_complete(ctx, FLASH_REG49 | FLASH_REG45);
            return;
        }
        match self.sd_handle_command(cmd_idx, arg) {
            None => {
                // CMD0: no response, command completes.
                if crate::native_mmio::sd_trace_active() {
                    crate::native_mmio::sd_log_cmd(cmd_idx, arg, 0, 0);
                }
                self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
                self.schedule_command_complete(ctx, FLASH_REG45);
            }
            Some((resp, long)) => {
                if long {
                    self.base.write_register(FLASH_REG8, resp[0]);
                    self.base.write_register(FLASH_REG9, resp[1]);
                    self.base.write_register(FLASH_REG10, resp[2]);
                    self.base.write_register(FLASH_REG11, resp[3]);
                } else {
                    self.base.write_register(FLASH_REG8, resp[0]);
                }
                if crate::native_mmio::sd_trace_active() {
                    crate::native_mmio::sd_log_cmd(
                        cmd_idx,
                        arg,
                        resp[0],
                        ((long as u32) << 16) | ((val & FLASH_REG31) >> 9),
                    );
                }
                self.base.clear_register_bits(FLASH_REG7, FLASH_REG28);
                if cmd_idx == 51 && self.current_block_number == 0xFFFF_FFFF {
                    // ACMD51 SEND_SCR: 8-byte SCR already staged in data_buffer.
                    self.push_scr_to_fifo(ctx);
                    self.schedule_command_complete(ctx, FLASH_REG45);
                } else if (val & FLASH_REG31) != 0 {
                    self.start_data_transfer(ctx, val);
                } else {
                    self.schedule_command_complete(ctx, FLASH_REG45);
                }
            }
        }
    }

    // ACMD51 SCR delivery: push the staged 8 bytes into the RX FIFO and flag
    // data-available + transfer-over (mirrors fill_fifo_from_card tail).
    fn push_scr_to_fifo(&mut self, ctx: &mut CpuContext) {
        for i in 0..2 {
            if self.fifo_count >= RSA_REG2 {
                break;
            }
            let idx = (i * 4) as usize;
            let w = u32::from_le_bytes([
                self.data_buffer[idx],
                self.data_buffer[idx + 1],
                self.data_buffer[idx + 2],
                self.data_buffer[idx + 3],
            ]);
            self.fifo[self.fifo_write_index as usize] = w;
            self.fifo_write_index = (self.fifo_write_index + 1) % RSA_REG2;
            self.fifo_count += 1;
        }
        self.blocks_remaining = 0;
        self.current_block_number = 0;
        self.base.set_register_bits(FLASH_REG13, FLASH_REG48 | FLASH_REG46);
        self.update_status();
        self.update_interrupt(ctx);
    }

    fn start_data_transfer(&mut self, ctx: &mut CpuContext, val: u32) {
        let data_len = self.base.read_register(FLASH_REG4);
        let block_size = self.base.read_register(FLASH_REG3);
        let ctrl = self.base.read_register(REG_FN);
        self.is_write_operation = (val & FLASH_REG30) != 0;
        self.data_length = data_len;
        self.data_offset = 0;
        let div = if block_size != 0 { block_size } else { 1 };
        self.blocks_remaining = (data_len + div - 1) / div;
        if self.ext_csd_pending {
            // MMC SEND_EXT_CSD staged the 512B in data_buffer (arg register
            // holds 0, not a block number).
            self.ext_csd_pending = false;
            self.current_block_number = Self::EXT_CSD_MARKER;
        } else {
            self.current_block_number = self.base.read_register(FLASH_REG6);
        }
        self.schedule_command_complete(ctx, FLASH_REG45);
        if (ctrl & FLASH_REG38) != 0 {
            self.start_dma_transfer(ctx);
        } else if !self.is_write_operation {
            self.fill_fifo_from_card(ctx);
        }
        self.update_interrupt(ctx);
    }

    fn fill_fifo_from_card(&mut self, ctx: &mut CpuContext) {
        let on_read = self.on_read_data;
        if on_read.is_none() {
            // Internal virtual card: SCR marker or a real 512B block.
            if self.current_block_number == 0xFFFF_FFFF {
                self.push_scr_to_fifo(ctx);
                return;
            }
            if self.current_block_number == Self::EXT_CSD_MARKER {
                // MMC SEND_EXT_CSD: stream the staged 512B (refills as the
                // FIFO drains, like read_fifo's top-up for real blocks).
                let avail = 512u32.saturating_sub(self.data_offset);
                let take = core::cmp::min(avail >> 2, RSA_REG2 - self.fifo_count);
                for i in 0..take {
                    let idx = (self.data_offset + i * 4) as usize;
                    let val = u32::from_le_bytes([
                        self.data_buffer[idx],
                        self.data_buffer[idx + 1],
                        self.data_buffer[idx + 2],
                        self.data_buffer[idx + 3],
                    ]);
                    self.fifo[self.fifo_write_index as usize] = val;
                    self.fifo_write_index = (self.fifo_write_index + 1) % RSA_REG2;
                    self.fifo_count += 1;
                }
                self.data_offset += take * 4;
                if self.data_offset >= 512 {
                    self.blocks_remaining = 0;
                    self.current_block_number = 0;
                }
                self.update_status();
                if self.fifo_count > 0 {
                    self.base.set_register_bits(FLASH_REG13, FLASH_REG48);
                }
                if self.blocks_remaining == 0 && self.fifo_count == 0 {
                    self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
                }
                self.update_interrupt(ctx);
                return;
            }
            let mut blk = [0u8; 512];
            Self::sd_read_block_internal(self.current_block_number, &mut blk);
            let block_size = self.base.read_register(FLASH_REG3);
            let take = core::cmp::min(block_size as usize, 512);
            let count = core::cmp::min((take >> 2) as u32, RSA_REG2 - self.fifo_count);
            for i in 0..count {
                let idx = (i * 4) as usize;
                let val = u32::from_le_bytes([
                    blk[idx],
                    blk[idx + 1],
                    blk[idx + 2],
                    blk[idx + 3],
                ]);
                self.fifo[self.fifo_write_index as usize] = val;
                self.fifo_write_index = (self.fifo_write_index + 1) % RSA_REG2;
                self.fifo_count += 1;
            }
            if count > 0 {
                self.current_block_number += 1;
                self.blocks_remaining = self.blocks_remaining.saturating_sub(1);
            }
            self.update_status();
            if self.fifo_count > 0 {
                self.base.set_register_bits(FLASH_REG13, FLASH_REG48);
            }
            if self.blocks_remaining == 0 && self.fifo_count == 0 {
                self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
            }
            self.update_interrupt(ctx);
            return;
        }
        let block_size = self.base.read_register(FLASH_REG3);
        if let Some(f) = on_read {
            let len = f(self.current_block_number, block_size, &mut self.data_buffer);
            if len > 0 {
                let count = core::cmp::min((len >> 2) as u32, RSA_REG2 - self.fifo_count);
                for i in 0..count {
                    let idx = (i * 4) as usize;
                    let val = u32::from_le_bytes([
                        self.data_buffer[idx],
                        self.data_buffer[idx + 1],
                        self.data_buffer[idx + 2],
                        self.data_buffer[idx + 3],
                    ]);
                    self.fifo[self.fifo_write_index as usize] = val;
                    self.fifo_write_index = (self.fifo_write_index + 1) % RSA_REG2;
                    self.fifo_count += 1;
                }
                self.current_block_number += 1;
                self.blocks_remaining -= 1;
            }
        }
        self.update_status();
        if self.fifo_count > 0 {
            self.base.set_register_bits(FLASH_REG13, FLASH_REG48);
        }
        if self.blocks_remaining == 0 && self.fifo_count == 0 {
            self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
        }
        self.update_interrupt(ctx);
    }

    fn drain_fifo_to_card(&mut self, ctx: &mut CpuContext) {
        let on_write = self.on_write_data;
        if on_write.is_none() {
            // Internal virtual card: pack FIFO words into 512B blocks.
            let block_size = self.base.read_register(FLASH_REG3);
            let words = (block_size >> 2).max(1);
            while self.fifo_count >= words && self.blocks_remaining > 0 {
                let mut blk = [0u8; 512];
                let take_words = core::cmp::min(words, 128);
                for i in 0..take_words {
                    let val = self.fifo[self.fifo_read_index as usize];
                    let idx = (i * 4) as usize;
                    blk[idx..idx + 4].copy_from_slice(&val.to_le_bytes());
                    self.fifo_read_index = (self.fifo_read_index + 1) % RSA_REG2;
                    self.fifo_count -= 1;
                }
                Self::sd_write_block_internal(self.current_block_number, &blk);
                self.current_block_number += 1;
                self.blocks_remaining = self.blocks_remaining.saturating_sub(1);
            }
            self.update_status();
            if self.fifo_count < RSA_REG2 && self.blocks_remaining > 0 {
                self.base.set_register_bits(FLASH_REG13, FLASH_REG47);
            }
            if self.blocks_remaining == 0 {
                self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
            }
            self.update_interrupt(ctx);
            return;
        }
        let block_size = self.base.read_register(FLASH_REG3);
        let words = block_size >> 2;
        while self.fifo_count >= words && self.blocks_remaining > 0 {
            let mut buf = [0u8; 4096];
            for i in 0..words {
                let val = self.fifo[self.fifo_read_index as usize];
                let idx = (i * 4) as usize;
                buf[idx..idx + 4].copy_from_slice(&val.to_le_bytes());
                self.fifo_read_index = (self.fifo_read_index + 1) % RSA_REG2;
                self.fifo_count -= 1;
            }
            if let Some(f) = on_write {
                f(self.current_block_number, &buf[..block_size as usize]);
            }
            self.current_block_number += 1;
            self.blocks_remaining -= 1;
        }
        self.update_status();
        if self.fifo_count < RSA_REG2 && self.blocks_remaining > 0 {
            self.base.set_register_bits(FLASH_REG13, FLASH_REG47);
        }
        if self.blocks_remaining == 0 {
            self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
        }
        self.update_interrupt(ctx);
    }

    fn read_fifo(&mut self, ctx: &mut CpuContext) -> u32 {
        if self.fifo_count == 0 {
            return 0;
        }
        let val = self.fifo[self.fifo_read_index as usize];
        self.fifo_read_index = (self.fifo_read_index + 1) % RSA_REG2;
        self.fifo_count -= 1;
        self.update_status();
        if !self.is_write_operation
            && self.fifo_count < RSA_REG2 / 2
            && self.blocks_remaining > 0
        {
            self.fill_fifo_from_card(ctx);
        }
        val
    }

    fn write_fifo(&mut self, ctx: &mut CpuContext, val: u32) {
        if self.fifo_count >= RSA_REG2 {
            return;
        }
        self.fifo[self.fifo_write_index as usize] = val;
        self.fifo_write_index = (self.fifo_write_index + 1) % RSA_REG2;
        self.fifo_count += 1;
        self.update_status();
        if self.is_write_operation && self.fifo_count >= RSA_REG2 / 2 {
            self.drain_fifo_to_card(ctx);
        }
    }

    fn start_dma_transfer(&mut self, ctx: &mut CpuContext) {
        if (self.base.read_register(FLASH_REG21) & FLASH_REG51) == 0 {
            return;
        }
        let desc_addr = self.base.read_register(FLASH_REG23);
        if desc_addr == 0 {
            self.base.set_register_bits(FLASH_REG24, FLASH_REG54 | FLASH_REG56);
            self.update_interrupt(ctx);
            return;
        }
        self.process_descriptor_chain(ctx, desc_addr);
    }

    fn resume_dma(&mut self, ctx: &mut CpuContext) {
        let desc_addr = self.base.read_register(FLASH_REG26);
        if desc_addr != 0 {
            self.process_descriptor_chain(ctx, desc_addr);
        }
    }

    fn process_descriptor_chain(&mut self, ctx: &mut CpuContext, addr: u32) {
        let block_size = self.base.read_register(FLASH_REG3);
        let blocks_remaining_at_start = self.blocks_remaining;
        if self.blocks_remaining <= 0 {
            return;
        }
        let mut tmp = addr;
        while tmp != 0 && self.blocks_remaining > 0 {
            let word0 = ctx.read_memory_u32(tmp);
            let word1 = ctx.read_memory_u32(tmp + 4);
            let word2 = ctx.read_memory_u32(tmp + 8);
            let word3 = ctx.read_memory_u32(tmp + 12);
            if (word0 & FLASH_REG57) == 0 {
                self.base.set_register_bits(FLASH_REG24, FLASH_REG54 | FLASH_REG56);
                self.base.write_register(FLASH_REG26, tmp);
                break;
            }
            let buf_size = word1 & 8191;
            let buf_addr = word2;
            let on_write = self.on_write_data;
            if self.is_write_operation {
                let mut buf = [0u8; 8192];
                for i in 0..buf_size {
                    buf[i as usize] = ctx.read_memory_u8(buf_addr + i);
                }
                if let Some(f) = on_write {
                    let num_blocks = (buf_size as u32 + block_size - 1) / block_size;
                    for blk in 0..num_blocks {
                        let start = (blk * block_size) as usize;
                        let end = core::cmp::min((blk + 1) * block_size, buf_size);
                        f(self.current_block_number, &buf[start..end as usize]);
                        self.current_block_number += 1;
                        self.blocks_remaining -= 1;
                    }
                } else if self.current_block_number != 0xFFFF_FFFF {
                    // Internal virtual card: split the DMA buffer into 512B blocks.
                    let num_blocks = (buf_size as u32 + block_size - 1) / block_size;
                    for blk in 0..num_blocks {
                        let mut blk_buf = [0u8; 512];
                        let start = (blk * block_size) as usize;
                        let end = core::cmp::min((blk + 1) * block_size, buf_size) as usize;
                        let len = end.saturating_sub(start).min(512);
                        blk_buf[..len].copy_from_slice(&buf[start..start + len]);
                        Self::sd_write_block_internal(self.current_block_number, &blk_buf);
                        self.current_block_number += 1;
                        self.blocks_remaining = self.blocks_remaining.saturating_sub(1);
                    }
                }
                self.base.set_register_bits(FLASH_REG24, FLASH_REG52 | FLASH_REG55);
            } else {
                let on_read = self.on_read_data;
                if let Some(f) = on_read {
                    let mut offset = 0u32;
                    while offset < buf_size && self.blocks_remaining > 0 {
                        let mut block_buf = [0u8; 512];
                        let len = f(self.current_block_number, block_size, &mut block_buf);
                        if len > 0 {
                            let copy_len = core::cmp::min(len as u32, buf_size - offset);
                            for j in 0..copy_len {
                                ctx.write_memory_u8(
                                    buf_addr + offset + j,
                                    block_buf[j as usize],
                                );
                            }
                            offset += copy_len;
                            self.current_block_number += 1;
                            self.blocks_remaining -= 1;
                        } else {
                            break;
                        }
                    }
                } else if self.current_block_number == 0xFFFF_FFFF {
                    // Internal virtual card: ACMD51 SCR (8 staged bytes).
                    let copy_len = core::cmp::min(8u32, buf_size);
                    for j in 0..copy_len {
                        ctx.write_memory_u8(buf_addr + j, self.data_buffer[j as usize]);
                    }
                    self.current_block_number = 0;
                    self.blocks_remaining = 0;
                } else if self.current_block_number == Self::EXT_CSD_MARKER {
                    // Internal virtual card: MMC SEND_EXT_CSD (512 staged bytes).
                    let copy_len = core::cmp::min(512u32, buf_size);
                    for j in 0..copy_len {
                        ctx.write_memory_u8(buf_addr + j, self.data_buffer[j as usize]);
                    }
                    self.current_block_number = 0;
                    self.blocks_remaining = 0;
                } else {
                    // Internal virtual card: real 512B blocks via host storage.
                    let mut offset = 0u32;
                    while offset < buf_size && self.blocks_remaining > 0 {
                        let mut block_buf = [0u8; 512];
                        Self::sd_read_block_internal(self.current_block_number, &mut block_buf);
                        let copy_len = core::cmp::min(512u32, buf_size - offset);
                        for j in 0..copy_len {
                            ctx.write_memory_u8(buf_addr + offset + j, block_buf[j as usize]);
                        }
                        offset += copy_len;
                        self.current_block_number += 1;
                        self.blocks_remaining = self.blocks_remaining.saturating_sub(1);
                    }
                }
                self.base.set_register_bits(FLASH_REG24, FLASH_REG53 | FLASH_REG55);
            }
            ctx.write_memory_u32(tmp, word0 & !FLASH_REG57);
            self.base.write_register(FLASH_REG26, tmp);
            if (word0 & FLASH_REG58) != 0 {
                tmp = self.base.read_register(FLASH_REG23);
            } else if (word0 & FLASH_REG59) != 0 {
                tmp = word3;
            } else {
                break;
            }
            if (word0 & FLASH_REG60) != 0 {
                break;
            }
        }
        if self.blocks_remaining == 0 && blocks_remaining_at_start > 0 {
            self.base.set_register_bits(FLASH_REG13, FLASH_REG46);
        }
        self.update_interrupt(ctx);
    }

    fn write_u8(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let aligned = addr & !3;
        let byte_off = addr & 3;
        let old = self.read_u32(ctx, aligned);
        let mask = 0xFFu32 << (8 * byte_off);
        self.write_u32(ctx, aligned, (old & !mask) | ((val & 0xFF) << (8 * byte_off)));
    }

    fn write_u16(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let aligned = addr & !3;
        let byte_off = addr & 3;
        let old = self.read_u32(ctx, aligned);
        let mask = 0xFFFFu32 << (8 * byte_off);
        self.write_u32(ctx, aligned, (old & !mask) | ((val & 0xFFFF) << (8 * byte_off)));
    }
}

impl MmioPeripheral for SdmmcPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr & 0xFFF;
        match offset {
            FLASH_REG16 | FLASH_REG17 => self.base.read_uint32(addr),
            FLASH_REG14 => {
                self.update_status();
                self.base.read_uint32(addr)
            }
            FLASH_REG12 => {
                self.base.read_register(FLASH_REG13) & self.base.read_register(FLASH_REG5)
            }
            _ => {
                if offset >= FLASH_REG27 && offset < FLASH_REG27 + 256 {
                    self.read_fifo(ctx)
                } else {
                    self.base.read_uint32(addr)
                }
            }
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        let offset = addr & 0xFFF;
        match offset {
            REG_FN => self.handle_ctrl_write(ctx, val),
            FLASH_REG7 => {
                self.base.write_uint32(addr, val);
                if (val & FLASH_REG28) != 0 {
                    self.start_command(ctx, val);
                }
            }
            FLASH_REG13 => {
                self.base.clear_register_bits(FLASH_REG13, val);
                self.update_interrupt(ctx);
            }
            FLASH_REG5 => {
                self.base.write_uint32(addr, val);
                self.update_interrupt(ctx);
            }
            FLASH_REG21 => self.handle_bmod_write(val),
            FLASH_REG22 => {
                if val != 0 {
                    self.resume_dma(ctx);
                }
            }
            FLASH_REG24 => {
                self.base.clear_register_bits(FLASH_REG24, val);
                self.update_interrupt(ctx);
            }
            FLASH_REG25 => {
                self.base.write_uint32(addr, val);
                self.update_interrupt(ctx);
            }
            FLASH_REG23 => self.base.write_uint32(addr, val),
            _ => {
                if offset >= FLASH_REG27 && offset < FLASH_REG27 + 256 {
                    self.write_fifo(ctx, val);
                } else {
                    self.base.write_uint32(addr, val);
                }
            }
        }
    }

    fn reset(&mut self) {
        self.base.reset();
        self.base.write_register(FLASH_REG19, 0x5432270a);
        self.base.write_register(FLASH_REG20, RSA_REG1);
        self.base.write_register(FLASH_REG2, 0xffffff40);
        self.base.write_register(FLASH_REG3, 512);
        self.base.write_register(FLASH_REG15, 2031616);
        self.base.write_register(FLASH_REG18, 0xffffff);
        // CLKSRC (offset 12): report both slots' card clocks as sourced
        // (source 1). The virtual card clock is always running (no gating
        // is modeled), and get_real_freq requires source==1. set_card_clk
        // overwrites this via RMW once it runs, so programmed flows are
        // unaffected; it only matters for reads before the first set_clk
        // (e.g. the eMMC max_freq=0 degenerate path, where EXT_CSD is
        // skipped and frequency is probed before any clock is programmed).
        self.base.write_register(12, 5);
        self.update_cdetect();
        self.fifo_read_index = 0;
        self.fifo_write_index = 0;
        self.fifo_count = 0;
        // Virtual card back to idle (host block storage is NOT wiped).
        self.sd_state = 0;
        self.sd_rca = 0;
        self.sd_selected = false;
        self.sd_blocklen = 512;
        self.sd_bus_width_4 = false;
        self.app_cmd_pending = false;
        // mmc_mode persists (card type config, re-applied by the host);
        // in-flight MMC state restarts.
        self.mmc_ocr_polls = 0;
        self.ext_csd_pending = false;
        self.mmc_bus_width = 0;
        self.update_status();
    }
}
