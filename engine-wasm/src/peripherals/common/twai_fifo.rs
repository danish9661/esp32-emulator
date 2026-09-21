use crate::peripherals::types::*;

const DS_REG45: u32 = 1;
const DS_REG46: u32 = 2;
const DS_REG47: u32 = 1;
const DS_REG48: u32 = 4;
const DS_REG49: u32 = 16;
const DS_REG50: u32 = 1;
const DS_REG51: u32 = 2;
const DS_REG52: u32 = 8;
const DS_REG53: u32 = 128;
const RSA_REG46: u32 = 64;

#[derive(Clone, Copy)]
pub struct CircularFifoBuffer {
    buf: [u32; 64],
    start: u32,
    used: u32,
}

impl CircularFifoBuffer {
    pub fn new() -> Self {
        CircularFifoBuffer { buf: [0; 64], start: 0, used: 0 }
    }

    pub fn available(&self) -> u32 {
        64 - self.used
    }

    pub fn push_byte(&mut self, val: u8) {
        if self.used < 64 {
            let idx = (self.start + self.used) % 64;
            self.buf[idx as usize] = val as u32;
            self.used += 1;
        }
    }

    pub fn pull(&mut self) -> u32 {
        if self.used == 0 { return 0; }
        let val = self.buf[self.start as usize];
        self.start = (self.start + 1) % 64;
        self.used -= 1;
        val
    }

    pub fn peek(&self, idx: u32) -> u32 {
        if idx < self.used {
            self.buf[((self.start + idx) % 64) as usize]
        } else {
            0
        }
    }

    pub fn item_count(&self) -> u32 {
        self.used
    }

    fn reset(&mut self) {
        self.used = 0;
        self.start = 0;
    }
}

#[derive(Clone, Copy)]
pub struct FieldDesc {
    pub reg_offset: u32,
    pub shift: u32,
    pub mask: u32,
}

impl FieldDesc {
    pub const fn new(reg_offset: u32, shift: u32, mask: u32) -> Self {
        FieldDesc { reg_offset, shift, mask }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CanMessage {
    pub(crate) ide: bool,
    pub(crate) rtr: bool,
    pub(crate) dlc: u32,
    pub(crate) int_status_alias: u32,
    pub(crate) data: [u8; 8],
}

#[derive(Clone, Copy)]
pub struct TwaiRegOffsets {
    pub data_0: u32,
    pub data_4: u32,
    pub data_8: u32,
    pub data_12: u32,
    pub status: u32,
    pub int_raw: u32,
    pub int_ena: u32,
    pub mode: u32,
    pub cmd: u32,
}

#[derive(Clone, Copy)]
pub struct TwaiFieldDescs {
    pub rx_filter_mode: FieldDesc,
    pub rx_message_counter: FieldDesc,
}

#[derive(Clone, Copy)]
pub struct TwaiConfig {
    pub rmt_channel_register: TwaiRegOffsets,
    pub f: TwaiFieldDescs,
}

pub struct TwaiPeripheral {
    base_addr: u32,
    index: u32,
    memory: [u32; 256],
    config: TwaiConfig,
    irq: u32,
    pub int_raw: u32,
    pub int_ena: u32,
    pub status: u32,
    loopback_message: Option<CanMessage>,
    rx_fifo: CircularFifoBuffer,
    filters: [u8; 8],
    // REAL-HW PARITY (SJA1000 STATUS bits): TBS is always set (synchronous
    // completion), TCS latches on completion and clears on the next TX_REQ,
    // RBS follows RX pending, DOS latches on RX overrun (CMD bit3 clears).
    tx_complete: bool,
    data_overrun: bool,
}

impl TwaiPeripheral {
    pub fn new(base_addr: u32, index: u32, config: TwaiConfig, irq: u32) -> Self {
        TwaiPeripheral {
            base_addr,
            index,
            memory: [0; 256],
            config,
            irq,
            int_raw: 0,
            int_ena: 0,
            status: 0,
            loopback_message: None,
            rx_fifo: CircularFifoBuffer::new(),
            filters: [0u8; 8],
            tx_complete: false,
            data_overrun: false,
        }
    }

    // Seed a register value (used by the host to apply Esp32FullResetValues
    // without going through MMIO side effects).
    pub fn seed_register(&mut self, offset: u32, val: u32) {
        self.memory[(offset >> 2) as usize] = val;
    }

    fn int_status(&self) -> u32 {
        self.int_raw & self.int_ena
    }

    fn reset_mode(&self) -> u32 {
        self.read_register(self.config.rmt_channel_register.mode) & DS_REG45
    }

    fn listen_only_mode(&self) -> u32 {
        self.read_register(self.config.rmt_channel_register.mode) & DS_REG46
    }

    fn read_register(&self, offset: u32) -> u32 {
        self.memory[(offset >> 2) as usize]
    }

    fn write_register(&mut self, offset: u32, val: u32) {
        self.memory[(offset >> 2) as usize] = val;
    }

    fn read_field(&self, desc: &FieldDesc) -> u32 {
        (self.read_register(desc.reg_offset) >> desc.shift) & desc.mask
    }

    fn write_field(&mut self, desc: &FieldDesc, val: u32) {
        let reg = self.read_register(desc.reg_offset);
        let mask_shifted = desc.mask << desc.shift;
        self.write_register(desc.reg_offset, (reg & !mask_shifted) | ((val & desc.mask) << desc.shift));
    }

    fn base_read_u32(&self, addr: u32) -> u32 {
        let offset = addr - self.base_addr;
        self.memory[(offset >> 2) as usize]
    }

    fn base_write_u32(&mut self, addr: u32, val: u32) {
        let offset = addr - self.base_addr;
        self.memory[(offset >> 2) as usize] = val;
    }

    fn base_reset(&mut self) {
        self.memory = [0; 256];
    }

    fn handle_transmit(&mut self, ctx: &mut CpuContext, cpu_val: bool) {
        // A new TX_REQ consumes the previous completion status.
        self.tx_complete = false;
        let tmp_val = &self.config.rmt_channel_register;
        let idx_val = self.read_register(tmp_val.data_0);
        let clock_event = tmp_val.data_0 + 4;
        let simulation_clock = tmp_val.data_0 + 8;
        let reg_val = tmp_val.data_0 + 12;
        let arg_val = tmp_val.data_0 + 16;
        let register_type = tmp_val.data_0 + 20;
        let cfg_val = (idx_val & DS_REG53) != 0;
        let h_val = (idx_val & RSA_REG46) != 0;
        let off_val = idx_val & 15;
        let val_val: u32;
        let flag: u32;
        if cfg_val {
            val_val = (self.read_register(clock_event) << 24) |
                      (self.read_register(simulation_clock) << 16) |
                      (self.read_register(reg_val) << 8) |
                      self.read_register(arg_val);
            flag = register_type;
        } else {
            val_val = (self.read_register(clock_event) << 3) |
                      ((self.read_register(simulation_clock) as i32 >> 5) as u32);
            flag = reg_val;
        }
        let mut len_val = [0u8; 8];
        if !h_val {
            for i in 0..off_val {
                len_val[i as usize] = self.read_register(flag + (i << 2)) as u8;
            }
        }
        let msg = CanMessage {
            ide: cfg_val,
            rtr: h_val,
            dlc: off_val,
            int_status_alias: val_val,
            data: len_val,
        };
        // SELF_RX_REQ always self-receives; plain TX_REQ self-receives in
        // SELF_TEST HW mode (MODE bit2 — what IDF NO_ACK mode uses).
        // Otherwise (NORMAL mode) the virtual CAN peer ACKs the frame: it
        // is captured to the TX mailbox (host-visible) and completes with
        // TCS exactly like a bus ACK — no self-reception (real-HW parity:
        // a transmitter never receives its own frame).
        let self_test = (self.read_register(tmp_val.mode) & 4) != 0;
        if cpu_val || self_test {
            self.loopback_message = Some(msg);
        } else {
            self.loopback_message = None;
            crate::native_mmio::twai_peer_capture(&msg);
        }
        self.transmit_complete(ctx);
    }

    /// Virtual-peer frame delivery (host-injected via native_twai_push_rx):
    /// runs the frame through the real acceptance filter + RX fifo so the
    /// driver's RX ISR path fires exactly as for bus-received frames.
    pub(crate) fn peer_receive(
        &mut self,
        ctx: &mut CpuContext,
        ide: bool,
        rtr: bool,
        dlc: u32,
        id: u32,
        data: [u8; 8],
    ) {
        let msg = CanMessage {
            ide,
            rtr,
            dlc: dlc.min(8),
            int_status_alias: id,
            data,
        };
        self.write_can_message(ctx, msg);
    }

    fn transmit_complete(&mut self, ctx: &mut CpuContext) {
        self.tx_complete = true;
        self.int_raw |= DS_REG51;
        self.update_interrupts(ctx);
        if let Some(msg) = self.loopback_message.take() {
            self.write_can_message(ctx, msg);
        }
    }

    fn filter_message(&self, msg: &CanMessage) -> bool {
        let tmp_val = self.read_field(&self.config.f.rx_filter_mode) == 0;
        let idx_val = if !msg.rtr && msg.dlc > 0 { msg.data[0] as u32 } else { 0 };
        let clock_event = if !msg.rtr && msg.dlc > 1 { msg.data[1] as u32 } else { 0 };
        if tmp_val {
            let f = &self.filters;
            let tmp_val = (((f[0] as u32) << 8 | f[1] as u32) << 8) | f[3] as u32;
            let clock_event = (f[2] as u32) << 8 | f[3] as u32;
            let simulation_clock = (((f[4] as u32) << 8 | f[5] as u32) << 8) | f[7] as u32;
            let reg_val = (f[6] as u32) << 8 | f[7] as u32;
            let arg_val = if msg.ide {
                (msg.int_status_alias >> 13) << 8
            } else {
                (msg.int_status_alias << 13) |
                (if msg.rtr { 4096 } else { 0 }) |
                ((240 & idx_val) << 4) |
                (15 & idx_val)
            };
            let register_type = if msg.ide {
                msg.int_status_alias >> 13
            } else {
                (msg.int_status_alias << 5) | (if msg.rtr { 16 } else { 0 })
            };
            (arg_val & !simulation_clock) == (tmp_val & !simulation_clock) ||
            (register_type & !reg_val) == (clock_event & !reg_val)
        } else {
            let f = &self.filters;
            let tmp_val = ((f[0] as u32) << 24) | ((f[1] as u32) << 16) | ((f[2] as u32) << 8) | f[3] as u32;
            let simulation_clock = ((f[4] as u32) << 24) | ((f[5] as u32) << 16) | ((f[6] as u32) << 8) | f[7] as u32;
            let masked = if msg.ide {
                (msg.int_status_alias << 3) | (if msg.rtr { 4 } else { 0 })
            } else {
                (msg.int_status_alias << 21) |
                (if msg.rtr { 1048576 } else { 0 }) |
                (idx_val << 8) |
                clock_event
            };
            (masked & !simulation_clock) == (tmp_val & !simulation_clock)
        }
    }

    fn write_can_message(&mut self, ctx: &mut CpuContext, msg: CanMessage) {
        if !self.filter_message(&msg) { return; }
        let tmp_val = msg.dlc + 1 + if msg.ide { 4 } else { 2 };
        self.update_counter(ctx, 1);
        if tmp_val > self.rx_fifo.available() {
            self.data_overrun = true;
            self.int_raw |= DS_REG52;
            self.update_interrupts(ctx);
        } else {
            let tmp_val = (if msg.ide { DS_REG53 } else { 0 }) |
                          (if msg.rtr { RSA_REG46 } else { 0 }) |
                          (15 & msg.dlc);
            self.rx_fifo.push_byte(tmp_val as u8);
            if msg.ide {
                self.rx_fifo.push_byte((msg.int_status_alias >> 24) as u8);
                self.rx_fifo.push_byte((msg.int_status_alias >> 16) as u8);
                self.rx_fifo.push_byte((msg.int_status_alias >> 8) as u8);
                self.rx_fifo.push_byte(msg.int_status_alias as u8);
            } else {
                self.rx_fifo.push_byte((msg.int_status_alias >> 3) as u8);
                self.rx_fifo.push_byte((msg.int_status_alias << 5) as u8);
            }
            if !msg.rtr {
                for i in 0..msg.dlc {
                    self.rx_fifo.push_byte(msg.data[i as usize]);
                }
            }
        }
    }

    fn next_message(&mut self, ctx: &mut CpuContext) {
        if self.rx_fifo.available() != 0 {
            let cpu_val = self.rx_fifo.pull();
            let tmp_val = (cpu_val & DS_REG53) != 0;
            let idx_val = (if (cpu_val & RSA_REG46) != 0 { 0 } else { 15 & cpu_val }) + if tmp_val { 4 } else { 2 };
            for _ in 0..idx_val {
                self.rx_fifo.pull();
            }
        }
        self.update_counter(ctx, -1);
    }

    fn update_counter(&mut self, ctx: &mut CpuContext, val: i32) {
        let rx_message_counter = self.config.f.rx_message_counter;
        let current = self.read_field(&rx_message_counter) as i32;
        let idx_val = (current + val).max(0).min(64) as u32;
        self.write_field(&rx_message_counter, idx_val);
        if idx_val != 0 {
            self.int_raw |= DS_REG50;
        } else {
            self.int_raw &= !DS_REG50;
        }
        self.update_interrupts(ctx);
    }

    // SJA1000 STATUS_REG parity: bit0 RBS (RX pending), bit1 DOS (overrun
    // latch), bit2 TBS (TX buffer released — always, synchronous model),
    // bit3 TCS (transmission complete latch).
    fn status_bits(&self) -> u32 {
        let rbs = if self.read_field(&self.config.f.rx_message_counter) != 0 { 1 } else { 0 };
        let dos = if self.data_overrun { 2 } else { 0 };
        let tbs = 4;
        let tcs = if self.tx_complete { 8 } else { 0 };
        rbs | dos | tbs | tcs
    }

    fn update_interrupts(&mut self, ctx: &mut CpuContext) {
        ctx.interrupt(self.irq, self.int_status() != 0);
    }

    fn on_mode_change(&mut self) {}
}

impl MmioPeripheral for TwaiPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let tmp_val = addr - self.base_addr;
        let idx_val = &self.config.rmt_channel_register;
        if tmp_val >= idx_val.data_0 && tmp_val <= idx_val.data_12 {
            let cpu_val = (tmp_val - idx_val.data_0) >> 2;
            if self.reset_mode() != 0 {
                return if cpu_val < 8 { self.filters[cpu_val as usize] as u32 } else { 0 };
            }
            return self.rx_fifo.peek(cpu_val);
        }
        match tmp_val {
            x if x == idx_val.status => self.status_bits(),
            x if x == idx_val.int_raw => {
                let cpu_val = self.int_raw;
                self.int_raw &= DS_REG50;
                // Re-evaluate the interrupt line: the matrix level stays
                // HIGH until explicitly lowered, so a consumed source with
                // no re-evaluation re-takes the ISR forever (500K+ empty
                // takes starved NORMAL-mode TX; NO_ACK only survived via
                // the RX-event yield + receive-side clear).
                self.update_interrupts(ctx);
                cpu_val
            }
            x if x == idx_val.int_ena => self.int_ena,
            _ => self.base_read_u32(addr),
        }
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, tmp_val: u32) {
        let idx_val = &self.config.rmt_channel_register;
        let clock_event = addr - self.base_addr;
        if clock_event >= idx_val.data_0 && clock_event <= idx_val.data_12 {
            let cpu_val = (clock_event - idx_val.data_0) >> 2;
            if self.reset_mode() != 0 {
                if cpu_val < 8 {
                    self.filters[cpu_val as usize] = tmp_val as u8;
                }
                return;
            }
            self.write_register(clock_event, tmp_val);
            return;
        }
        match clock_event {
            x if x == idx_val.mode => {
                if tmp_val != self.read_register(idx_val.mode) {
                    self.write_register(idx_val.mode, tmp_val);
                    self.on_mode_change();
                }
                return;
            }
            x if x == idx_val.cmd => {
                if tmp_val & DS_REG47 != 0 { self.handle_transmit(ctx, false); }
                if tmp_val & DS_REG48 != 0 { self.next_message(ctx); }
                if tmp_val & DS_REG49 != 0 { self.handle_transmit(ctx, true); }
                if tmp_val & 8 != 0 {
                    self.data_overrun = false;
                    self.int_raw &= !DS_REG52;
                    self.update_interrupts(ctx);
                }
                return;
            }
            x if x == idx_val.int_ena => {
                self.int_ena = tmp_val;
                self.base_write_u32(addr, tmp_val);
                self.update_interrupts(ctx);
            }
            _ => {}
        }
        self.base_write_u32(addr, tmp_val);
    }

    fn reset(&mut self) {
        self.base_reset();
        self.rx_fifo.reset();
        self.tx_complete = false;
        self.data_overrun = false;
    }
}
