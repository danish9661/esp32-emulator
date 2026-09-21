use crate::peripherals::types::*;
use crate::peripherals::common::peripheral::PeripheralBase;

const LED_REG19: u32 = 0;
const LED_REG20: u32 = 255;
const LED_REG21: u32 = 24;
const LED_REG22: u32 = 15;
const LED_REG23: u32 = 1;
const LED_REG24: u32 = 4;
const LED_REG25: u32 = 8;
const LED_REG26: u32 = 32;
const LED_REG27: u32 = 64;
const LED_REG28: u32 = 131072;
const LED_REG29: u32 = 262144;
const LED_REG30: u32 = 524288;
const LED_REG31: u32 = 2;
const LED_REG32: u32 = 4;
const LED_REG33: u32 = 255;
const LED_REG34: u32 = 12;
const LED_REG35: u32 = 63;
const LED_REG36: u32 = 18;
const LED_REG37: u32 = 63;
const LED_REG38: u32 = 24;
const LED_REG39: u32 = 3;
const LED_REG40: u32 = 0;
const LED_REG41: u32 = 24;
const LED_REG42: u32 = 1;
const LED_REG43: u32 = 2;
const LED_REG44: u32 = 8;
const LED_REG45: u32 = 16;
const LED_REG46: u32 = 32;
const LED_REG47: u32 = 64;
const LED_REG48: u32 = 8;
const LED_REG49: u32 = 255;
const LED_REG50: u32 = 16;
const LED_REG51: u32 = 15;
const LED_REG52: u32 = 8;

const RMT_CLOCK_SOURCE_APB: u32 = 1;
const RMT_CLOCK_SOURCE_RC_FAST: u32 = 2;
const RMT_CLOCK_SOURCE_XTAL: u32 = 3;

const RMT_CHANNEL_REGISTER_CONF0: u32 = 0;
const RMT_CHANNEL_REGISTER_CONF1: u32 = 1;
const RMT_CHANNEL_REGISTER_TX_LIM: u32 = 2;

const RMT_EVENT_BASE: u32 = 200;

const UART_REG1: u32 = 0;
const UART_REG2: u32 = 512;
const UART_REG3: u32 = 1024;
const UART_REG4: u32 = 1536;
const UART_REG5: u32 = 2048;
const UART_REG6: u32 = 2052;
const UART_REG7: u32 = 2056;
const UART_REG8: u32 = 2060;
const UART_REG9: u32 = 2064;
const UART_REG10: u32 = 2068;
const UART_REG11: u32 = 2072;

#[derive(Clone, Copy)]
pub struct RmtChannelConfig {
    pub v2: bool,
    pub conf0: u32,
    pub conf1: i32,
    pub duty: u32,
    pub tx_lim: u32,
    pub matrix_out: u32,
    pub ram_block_size: u32,
}

pub struct RmtChannel {
    // Back-pointer to the parent peripheral. NOTE: RmtPeripheral::new()
    // builds into a local and the value is moved into its static home
    // afterwards, so this MUST be re-pointed post-move (see the fixup in
    // native_rmt_init) — the address taken inside new() dangles.
    pub rmt: *mut RmtPeripheral,
    index: u32,
    config: RmtChannelConfig,
    mem_size: u32,
    mem_start: u32,
    rx_offset: u32,
    tx_offset: u32,
    tx_counter: u32,
    counter_wrapped: u32,
    continuous: u32,
    wraparound: u32,
    tx_threshold: u32,
    rx_owner: u32,
    tx_enabled: u32,
    // Virtual-wire RX: set when the driver arms the channel (CONF1 RX_EN,
    // bit 1); a TX completion on any other channel streams its items into
    // this channel's RAM and raises RX_END (single-chip loopback parity
    // with the SPI/I2C virtual buses).
    rx_enabled: u32,
    idle_output_enable: u32,
    idle_level: u32,
    event_id: usize,
    clock_divider: u32,
    clock_parent_is_ref: u32,
}

impl RmtChannel {
    pub fn new(rmt: *mut RmtPeripheral, index: u32, config: RmtChannelConfig) -> Self {
        let mem_size = 4 * config.ram_block_size;
        let mem_start = index * mem_size;
        let mut ch = RmtChannel {
            rmt,
            index,
            config,
            mem_size,
            mem_start,
            rx_offset: 0,
            tx_offset: 0,
            tx_counter: 0,
            counter_wrapped: 0,
            continuous: 0,
            wraparound: 0,
            tx_threshold: 128,
            rx_owner: 1,
            rx_enabled: 0,
            tx_enabled: 0,
            idle_output_enable: 0,
            idle_level: 0,
            event_id: usize::MAX,
            clock_divider: 1,
            clock_parent_is_ref: 0,
        };
        ch.reset();
        ch
    }

    fn clock_freq_hz(&self, ctx: &CpuContext) -> u64 {
        if self.clock_parent_is_ref != 0 {
            let parent_freq = ctx.clocks.ref_tick.frequency as u64;
            if self.clock_divider == 0 { parent_freq } else { parent_freq / self.clock_divider as u64 }
        } else {
            let rmt = unsafe { &*self.rmt };
            if rmt.v2 != 0 {
                let parent_freq = rmt.own_rmt_clock_freq as u64;
                if self.clock_divider == 0 { parent_freq } else { parent_freq / self.clock_divider as u64 }
            } else {
                let parent_freq = ctx.clocks.apb.frequency as u64;
                if self.clock_divider == 0 { parent_freq } else { parent_freq / self.clock_divider as u64 }
            }
        }
    }

    fn schedule_transmit_next(&mut self, ctx: &mut CpuContext, delta_ticks: u32) {
        let freq = self.clock_freq_hz(ctx);
        let delta_ns = if freq == 0 { 0 } else { (delta_ticks as u64 * 1_000_000_000) / freq };
        self.event_id = ctx.schedule_event(delta_ns, EventTag::FrcTimerAlarm { channel: RMT_EVENT_BASE + self.index });
    }

    pub fn start_tx(&mut self, ctx: &mut CpuContext) {
        if self.tx_enabled == 0 {
            self.tx_enabled = 1;
            self.tx_offset = 0;
            self.tx_counter = 0;
            self.counter_wrapped = 0;
            unsafe { (*self.rmt).transmit_start(ctx) };
            self.transmit_next(ctx);
        }
    }

    pub fn transmit_done(&mut self, ctx: &mut CpuContext) {
        let irq_offset = if self.config.v2 { LED_REG40 + self.index } else { LED_REG40 + 3 * self.index };
        unsafe { (*self.rmt).set_interrupt(ctx, irq_offset) };
        // Virtual wire: stream the transmitted items into every armed RX
        // channel before raising TX_END (single ISR services both).
        let (idx, start, nbytes) = (self.index, self.mem_start, self.tx_offset);
        unsafe { (*self.rmt).deliver_loopback(ctx, idx, start, nbytes) };
        ctx.gpio_matrix.set_output(self.config.matrix_out, self.idle_output_enable != 0, self.idle_level != 0);
        if self.continuous != 0 {
            self.tx_offset = 0;
            self.schedule_transmit_next(ctx, 1);
        } else {
            self.tx_enabled = 0;
            unsafe { (*self.rmt).transmit_done(ctx) };
        }
    }

    pub fn tx_active(&self) -> bool {
        self.tx_enabled != 0
    }

    pub fn write_conf0(&mut self, _ctx: &mut CpuContext, cpu_val: u32) {
        self.mem_size = 4 * self.config.ram_block_size * ((cpu_val >> LED_REG21) & LED_REG22);
        let tmp_val = (cpu_val >> LED_REG19) & LED_REG20;
        self.clock_divider = if tmp_val > 0 { tmp_val } else { 256 };
        unsafe { (*self.rmt).base.write_register(self.config.conf0, cpu_val) };
    }

    pub fn write_conf1(&mut self, ctx: &mut CpuContext, cpu_val: u32) {
        if cpu_val & LED_REG25 != 0 {
            self.tx_offset = 0;
            self.tx_counter = 0;
            self.counter_wrapped = 0;
        }
        if cpu_val & LED_REG24 != 0 {
            self.rx_offset = 0;
        }
        self.rx_owner = if cpu_val & LED_REG26 != 0 { 1 } else { 0 };
        // CONF1 bit 1 = RX_EN (rmt_reg.h RMT_RX_EN_CHn): arm/disarm the
        // virtual-wire receiver.
        self.rx_enabled = if cpu_val & 2 != 0 { 1 } else { 0 };
        if cpu_val & LED_REG28 != 0 {
            self.clock_parent_is_ref = 0;
        } else {
            if ctx.clocks.ref_tick.frequency == 0 { panic!("Ref clock unavailable!"); }
            self.clock_parent_is_ref = 1;
        }
        self.continuous = if cpu_val & LED_REG27 != 0 { 1 } else { 0 };
        self.idle_output_enable = if cpu_val & LED_REG30 != 0 { 1 } else { 0 };
        self.idle_level = if cpu_val & LED_REG29 != 0 { 1 } else { 0 };
        unsafe { (*self.rmt).base.write_register(self.config.conf1 as u32, cpu_val & !(LED_REG25 | LED_REG24 | LED_REG23)) };
        if cpu_val & LED_REG23 != 0 {
            self.start_tx(ctx);
        } else {
            self.tx_enabled = 0;
        }
    }

    pub fn write_conf0v2(&mut self, ctx: &mut CpuContext, cpu_val: u32) {
        if cpu_val & LED_REG43 != 0 {
            self.tx_offset = 0;
            self.tx_counter = 0;
            self.counter_wrapped = 0;
        }
        self.continuous = if cpu_val & LED_REG44 != 0 { 1 } else { 0 };
        self.idle_output_enable = if cpu_val & LED_REG47 != 0 { 1 } else { 0 };
        self.idle_level = if cpu_val & LED_REG46 != 0 { 1 } else { 0 };
        self.wraparound = if cpu_val & LED_REG45 != 0 { 1 } else { 0 };
        let tmp_val = (cpu_val >> LED_REG50) & LED_REG51;
        self.mem_size = 4 * self.config.ram_block_size * tmp_val;
        let idx_val = (cpu_val >> LED_REG48) & LED_REG49;
        self.clock_divider = if idx_val > 0 { idx_val } else { 256 };
        unsafe { (*self.rmt).base.write_register(self.config.conf0, cpu_val & !(LED_REG43 | LED_REG23)) };
        if cpu_val & LED_REG42 != 0 {
            self.start_tx(ctx);
        } else {
            self.tx_enabled = 0;
        }
    }

    pub fn write_tx_lim(&mut self, _ctx: &mut CpuContext, cpu_val: u32) {
        self.tx_threshold = 511 & cpu_val;
        unsafe { (*self.rmt).base.write_register(self.config.tx_lim, cpu_val) };
    }

    pub fn set_wraparound(&mut self, cpu_val: bool) {
        self.wraparound = if cpu_val { 1 } else { 0 };
    }

    pub fn reset(&mut self) {
        self.wraparound = 0;
        self.tx_counter = 0;
        self.tx_offset = 0;
        self.rx_enabled = 0;
        self.tx_enabled = 0;
    }

    pub fn transmit_next(&mut self, ctx: &mut CpuContext) {
        let cpu_val = unsafe { (*self.rmt).read_ram((self.mem_start + self.tx_offset) & !3) };
        let tmp_val = if self.tx_offset % 4 == 2 { cpu_val >> 16 } else { cpu_val } & 65535;
        let idx = 32768 & tmp_val != 0;
        let clock_event = 32767 & tmp_val;
        if clock_event == 0 {
            self.transmit_done(ctx);
            return;
        }
        if self.counter_wrapped != 0 {
            let cpu = if self.config.v2 { LED_REG52 } else { LED_REG41 };
            unsafe { (*self.rmt).set_interrupt(ctx, cpu + self.index) };
            self.counter_wrapped = 0;
        }
        ctx.gpio_matrix.set_output(self.config.matrix_out, true, idx);
        self.tx_offset += 2;
        self.tx_counter += 1;
        if self.tx_offset >= self.mem_size {
            if self.wraparound == 0 { self.transmit_done(ctx); return; }
            self.tx_offset = 0;
        }
        if self.tx_counter == 2 * self.tx_threshold {
            self.counter_wrapped = 1;
            self.tx_counter = 0;
        }
        self.schedule_transmit_next(ctx, clock_event);
    }

    pub fn handle_event(&mut self, ctx: &mut CpuContext) {
        self.transmit_next(ctx);
    }
}

pub struct RegisterMapEntry {
    pub offset: u32,
    pub channel: u32,
    pub reg_type: u32,
}

#[derive(Clone, Copy)]
pub struct RmtPeripheralConfig {
    pub channels: u32,
    pub ch0_matrix_out: u32,
    pub ram_block_size: u32,
    pub ram_start: u32,
    pub ram_size: u32,
    pub irq: u32,
    pub int_raw: u32,
    pub int_ena: u32,
    pub int_st: u32,
    pub int_clr: u32,
    pub sys_conf: u32,
    pub apb_conf: u32,
    pub ch0_conf0: i32,
    pub ch0_conf1: i32,
    pub ch0_carrier_duty: u32,
    pub ch0_tx_lim: u32,
    pub ch0_tx_conf0: u32,
    pub clock_source: u32,
}

pub struct RmtPeripheral {
    pub base: PeripheralBase,
    pub config: RmtPeripheralConfig,
    pub tx_state: u32,
    pub channels: [RmtChannel; 8],
    pub channel_count: u32,
    pub register_map: [RegisterMapEntry; 32],
    pub register_map_count: u32,
    pub v2: u32,
    pub own_rmt_clock_freq: u32,
}

impl RmtPeripheral {
    pub fn new(base_addr: u32, name: &'static str, config: RmtPeripheralConfig) -> Self {
        let v2 = if config.ch0_conf1 < 0 { 1u32 } else { 0u32 };
        let channel_count = config.channels;
        let reg_step = if v2 != 0 { 4u32 } else { 8u32 };
        let mut reg_map_count = 0u32;

        let base = PeripheralBase::new(base_addr, name);

        let channels: [RmtChannel; 8] = unsafe { core::mem::zeroed() };
        let register_map: [RegisterMapEntry; 32] = unsafe { core::mem::zeroed() };

        let mut rmt = RmtPeripheral {
            base,
            config,
            tx_state: 0,
            channels,
            channel_count: 0,
            register_map,
            register_map_count: 0,
            v2,
            own_rmt_clock_freq: 80000000,
        };

        for c in 0..channel_count as usize {
            let conf0_offset = if config.ch0_conf0 >= 0 { config.ch0_conf0 as u32 } else { config.ch0_tx_conf0 } + reg_step * c as u32;
            let conf1_offset_val = if v2 != 0 { 0xFFFFFFFF } else { config.ch0_conf1 as u32 + reg_step * c as u32 };
            let duty_offset = config.ch0_carrier_duty + (c as u32) * 4;
            let tx_lim_offset = config.ch0_tx_lim + (c as u32) * 4;
            let ch_config = RmtChannelConfig {
                v2: v2 != 0,
                conf0: conf0_offset,
                conf1: if v2 != 0 { -1 } else { conf1_offset_val as i32 },
                duty: duty_offset,
                tx_lim: tx_lim_offset,
                matrix_out: config.ch0_matrix_out + c as u32,
                ram_block_size: config.ram_block_size,
            };
            let channel_ptr: *mut RmtPeripheral = &mut rmt as *mut RmtPeripheral;
            rmt.channels[c] = RmtChannel::new(channel_ptr, c as u32, ch_config);
            let mut ri = reg_map_count as usize;
            rmt.register_map[ri] = RegisterMapEntry { offset: conf0_offset, channel: c as u32, reg_type: RMT_CHANNEL_REGISTER_CONF0 };
            ri += 1;
            if v2 == 0 {
                rmt.register_map[ri] = RegisterMapEntry { offset: conf1_offset_val, channel: c as u32, reg_type: RMT_CHANNEL_REGISTER_CONF1 };
                ri += 1;
            }
            rmt.register_map[ri] = RegisterMapEntry { offset: tx_lim_offset, channel: c as u32, reg_type: RMT_CHANNEL_REGISTER_TX_LIM };
            ri += 1;
            reg_map_count = ri as u32;
        }
        rmt.channel_count = channel_count;
        rmt.register_map_count = reg_map_count;
        rmt
    }

    pub fn dma_set_in(&mut self, _cpu_val: u32) {}

    pub fn dma_set_out(&mut self, _cpu_val: u32) {}

    pub fn int_status(&self) -> u32 {
        self.base.read_register(self.config.int_raw) & self.base.read_register(self.config.int_ena)
    }

    pub fn read_ram(&self, addr: u32) -> u32 {
        self.base.read_register(self.config.ram_start + (addr % self.config.ram_size))
    }

    pub fn set_interrupt(&mut self, ctx: &mut CpuContext, cpu_val: u32) {
        self.base.set_register_bits(self.config.int_raw, 1 << cpu_val);
        if self.int_status() != 0 { ctx.interrupt(self.config.irq, true); }
    }

    pub fn update_tx_state(&mut self) {
        let mut active = 0u32;
        for i in 0..self.channel_count as usize {
            if self.channels[i].tx_active() { active = 1; break; }
        }
        if active != self.tx_state { self.tx_state = active; }
    }

    pub fn transmit_start(&mut self, _ctx: &mut CpuContext) { self.update_tx_state(); }
    pub fn transmit_done(&mut self, _ctx: &mut CpuContext) { self.update_tx_state(); }

    pub fn write_ram(&mut self, addr: u32, val: u32) {
        self.base.write_register(self.config.ram_start + (addr % self.config.ram_size), val);
    }

    // Virtual-wire RX delivery: copy a completed TX payload into every
    // armed (RX_EN) sibling channel's RAM, flip its MEM_OWNER to SW, and
    // raise RX_END. One-shot per arming (the driver re-arms via rx_start).
    pub fn deliver_loopback(&mut self, ctx: &mut CpuContext, src_idx: u32, src_start: u32, nbytes: u32) {
        // Include the zero terminator word the RX ISR scans for.
        let total = nbytes + 4;
        for j in 0..self.channel_count as usize {
            if j as u32 == src_idx || self.channels[j].rx_enabled == 0 { continue; }
            let (dst_start, dst_size) = (self.channels[j].mem_start, self.channels[j].mem_size);
            let nwords = core::cmp::min((total + 3) / 4, dst_size / 4) as usize;
            for w in 0..nwords {
                let v = self.read_ram((src_start + (w as u32) * 4) & !3);
                let base = self.config.ram_start;
                let size = self.config.ram_size;
                self.base.write_register(base + ((dst_start + (w as u32) * 4) % size), v);
            }
            self.channels[j].rx_enabled = 0;
            self.channels[j].rx_offset = 0;
            // MEM_OWNER (CONF1 bit 5) flips to SW on RX_END, like hardware.
            let conf1 = self.channels[j].config.conf1;
            if conf1 >= 0 {
                let v = self.base.read_register(conf1 as u32);
                self.base.write_register(conf1 as u32, v | 32);
            }
            // CHnSTATUS MEM_WADDR_EX[9:0]: absolute received word address —
            // the RX ISR derives the byte length as (waddr - ch*64) * 4.
            // Without it the length underflows and the ringbuffer send fails.
            let waddr = dst_start / 4 + nwords as u32;
            self.base.write_register(0x60 + 4 * j as u32, waddr & 0x3FF);
            let irq = if self.v2 != 0 { LED_REG40 + j as u32 } else { LED_REG40 + 3 * j as u32 + 1 };
            self.set_interrupt(ctx, irq);
        }
    }

    fn find_channel(&self, offset: u32) -> Option<(u32, u32)> {
        for i in 0..self.register_map_count as usize {
            if self.register_map[i].offset == offset {
                return Some((self.register_map[i].channel, self.register_map[i].reg_type));
            }
        }
        None
    }
}

impl MmioPeripheral for RmtPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr & 0xFFF;
        if offset == self.config.int_st { return self.int_status(); }
        self.base.read_uint32(addr)
    }

    fn write_u32(&mut self, ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        let offset = addr & 0xFFF;
        if let Some((ch_idx, reg_type)) = self.find_channel(offset) {
            let ch = &mut self.channels[ch_idx as usize];
            match reg_type {
                RMT_CHANNEL_REGISTER_CONF0 => {
                    if self.v2 != 0 { ch.write_conf0v2(ctx, val); } else { ch.write_conf0(ctx, val); }
                }
                RMT_CHANNEL_REGISTER_CONF1 => { ch.write_conf1(ctx, val); }
                RMT_CHANNEL_REGISTER_TX_LIM => { ch.write_tx_lim(ctx, val); }
                _ => {}
            }
            return;
        }
        let cfg = &self.config;
        if offset == cfg.int_raw || offset == cfg.int_st { return; }
        if offset == cfg.int_clr {
            self.base.clear_register_bits(cfg.int_raw, val);
            ctx.interrupt(cfg.irq, self.int_status() != 0);
            return;
        }
        if offset == cfg.sys_conf {
            let reg_val = (val >> LED_REG38) & LED_REG39;
            let arg_val = (val >> LED_REG32) & LED_REG33;
            let register_type = (val >> LED_REG34) & LED_REG35;
            let cfg_val = (val >> LED_REG36) & LED_REG37;
            let base_freq = match reg_val {
                RMT_CLOCK_SOURCE_APB => ctx.clocks.apb.frequency,
                RMT_CLOCK_SOURCE_RC_FAST => ctx.clocks.rc_fast.frequency,
                RMT_CLOCK_SOURCE_XTAL => ctx.clocks.xtal.frequency,
                _ => ctx.clocks.apb.frequency,
            };
            let own_div_num = (arg_val + 1) as u64 * cfg_val as u64 + register_type as u64;
            let own_div_den = cfg_val as u64;
            if own_div_num > 0 && own_div_den > 0 {
                self.own_rmt_clock_freq = ((base_freq as u64 * own_div_den / own_div_num) as u32);
            } else {
                self.own_rmt_clock_freq = base_freq;
            }
            return;
        }
        if offset == cfg.apb_conf {
            let wrap = val & LED_REG31 != 0;
            for i in 0..self.channel_count as usize { self.channels[i].set_wraparound(wrap); }
            return;
        }
    }

    fn reset(&mut self) {
        self.base.reset();
        self.tx_state = 0;
        for i in 0..self.channel_count as usize { self.channels[i].reset(); }
    }
}

pub struct RngPeripheral {
    pub base: PeripheralBase,
    pub data_offset: u32,
}

impl RngPeripheral {
    pub fn new(base_addr: u32, name: &'static str, data_offset: u32) -> Self {
        RngPeripheral { base: PeripheralBase::new(base_addr, name), data_offset }
    }

    pub fn generate_random_uint32(&mut self, _ctx: &mut CpuContext) -> u32 {
        crate::peripherals::common::helpers::random_u32()
    }
}

impl MmioPeripheral for RngPeripheral {
    fn read_u32(&mut self, ctx: &mut CpuContext, addr: u32) -> u32 {
        let offset = addr & 0xFFF;
        if offset == self.data_offset { return self.generate_random_uint32(ctx); }
        self.base.read_uint32(addr)
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
    }

    fn reset(&mut self) { self.base.reset(); }
}

const RSA_LIMBS: usize = 64;

#[derive(Clone, Copy)]
struct BigUint {
    limbs: [u64; RSA_LIMBS],
}

impl BigUint {
    fn zero() -> Self { BigUint { limbs: [0u64; RSA_LIMBS] } }

    fn one() -> Self {
        let mut r = BigUint::zero();
        r.limbs[0] = 1;
        r
    }

    fn from_bytes(buf: &[u8]) -> Self {
        let mut r = BigUint::zero();
        let n = buf.len().min(RSA_LIMBS * 8);
        for i in 0..n {
            r.limbs[i / 8] |= (buf[i] as u64) << ((i % 8) * 8);
        }
        r
    }

    fn to_bytes(&self, buf: &mut [u8], len: usize) {
        let n = len.min(RSA_LIMBS * 8);
        for i in 0..n {
            buf[i] = (self.limbs[i / 8] >> ((i % 8) * 8)) as u8;
        }
        for i in n..len { buf[i] = 0; }
    }

    fn is_zero(&self) -> bool {
        let mut i = 0;
        while i < RSA_LIMBS { if self.limbs[i] != 0 { return false; } i += 1; }
        true
    }

    fn is_odd(&self) -> bool { self.limbs[0] & 1 != 0 }

    fn shr32(&mut self) {
        for i in 0..RSA_LIMBS - 1 { self.limbs[i] = (self.limbs[i] >> 32) | (self.limbs[i + 1] << 32); }
        self.limbs[RSA_LIMBS - 1] >>= 32;
    }

    fn add(&self, other: &BigUint) -> BigUint {
        let mut r = BigUint::zero();
        let mut carry = 0u64;
        for i in 0..RSA_LIMBS {
            let (sum, c1) = self.limbs[i].overflowing_add(other.limbs[i]);
            let (sum2, c2) = sum.overflowing_add(carry);
            r.limbs[i] = sum2;
            carry = (c1 as u64) + (c2 as u64);
        }
        r
    }

    fn sub(&self, other: &BigUint) -> BigUint {
        let mut r = BigUint::zero();
        let mut borrow = 0u64;
        for i in 0..RSA_LIMBS {
            let (diff, b1) = self.limbs[i].overflowing_sub(other.limbs[i]);
            let (diff2, b2) = diff.overflowing_sub(borrow);
            r.limbs[i] = diff2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        r
    }

    fn mul_wide(&self, other: &BigUint) -> BigUint {
        let mut r = BigUint::zero();
        for i in 0..RSA_LIMBS {
            if self.limbs[i] == 0 { continue; }
            let mut carry = 0u128;
            for j in 0..RSA_LIMBS - i {
                let prod = (self.limbs[i] as u128) * (other.limbs[j] as u128);
                let (sum, c1) = (r.limbs[i + j] as u128).overflowing_add(prod);
                let (sum2, c2) = sum.overflowing_add(carry);
                r.limbs[i + j] = sum2 as u64;
                carry = (c1 as u128) + (c2 as u128) + (sum2 >> 64);
            }
        }
        r
    }

    fn ge(&self, other: &BigUint) -> bool {
        let mut i = RSA_LIMBS;
        while i > 0 {
            i -= 1;
            if self.limbs[i] > other.limbs[i] { return true; }
            if self.limbs[i] < other.limbs[i] { return false; }
        }
        true
    }

    fn rem(&self, modulus: &BigUint) -> BigUint {
        if modulus.is_zero() { return *self; }
        let mut r = *self;
        if !r.ge(modulus) { return r; }
        let ms = *modulus;
        let mut ms_bit = RSA_LIMBS * 64;
        while ms_bit > 0 {
            ms_bit -= 1;
            if (ms.limbs[ms_bit / 64] >> (ms_bit % 64)) & 1 != 0 { break; }
        }
        let mut rs_bit = RSA_LIMBS * 64;
        while rs_bit > 0 {
            rs_bit -= 1;
            if (r.limbs[rs_bit / 64] >> (rs_bit % 64)) & 1 != 0 { break; }
        }
        while rs_bit >= ms_bit && rs_bit > 0 {
            let shift = rs_bit - ms_bit;
            let mut shifted = *modulus;
            for _ in 0..shift { shifted.shl1(); }
            if r.ge(&shifted) { r = r.sub(&shifted); }
            rs_bit -= 1;
        }
        while r.ge(modulus) { r = r.sub(modulus); }
        r
    }

    fn shl1(&mut self) {
        let mut carry = 0u64;
        for i in 0..RSA_LIMBS {
            let next_carry = self.limbs[i] >> 63;
            self.limbs[i] = (self.limbs[i] << 1) | carry;
            carry = next_carry;
        }
    }

    fn shr1(&mut self) {
        for i in (0..RSA_LIMBS - 1).rev() { self.limbs[i] = (self.limbs[i] >> 1) | (self.limbs[i + 1] << 63); }
        self.limbs[RSA_LIMBS - 1] >>= 1;
    }

    fn mod_mul(&self, other: &BigUint, modulus: &BigUint) -> BigUint {
        let prod = self.mul_wide(other);
        prod.rem(modulus)
    }

    fn mod_pow(base: &BigUint, exp: &BigUint, modulus: &BigUint) -> BigUint {
        let mut result = BigUint::one();
        let mut b = *base;
        let mut e = *exp;
        while !e.is_zero() {
            if e.is_odd() { result = result.mod_mul(&b, modulus); }
            e.shr1();
            b = b.mod_mul(&b, modulus);
        }
        result.rem(modulus)
    }

    fn mod_inverse(a: &BigUint, m: &BigUint) -> BigUint {
        if a.is_zero() || m.is_zero() { return BigUint::zero(); }
        let mut t = BigUint::zero();
        let mut new_t = BigUint::one();
        let mut r = *m;
        let mut new_r = *a;
        while !new_r.is_zero() {
            let quotient = r.div(&new_r);
            let temp_r = r;
            r = new_r;
            new_r = temp_r.sub(&quotient.mul_wide(&new_r)).rem(&temp_r.add(m));
            let temp_t = t;
            t = new_t;
            new_t = temp_t.sub(&quotient.mul_wide(&new_t)).rem(&temp_t.add(m));
        }
        if r.ge(m) { r = r.sub(m); }
        r
    }

    fn div(&self, other: &BigUint) -> BigUint {
        if other.is_zero() { return BigUint::zero(); }
        let mut q = BigUint::zero();
        let mut r = *self;
        let mut shift = 0i32;
        let mut shifted = *other;
        while r.ge(&shifted) { shifted.shl1(); shift += 1; }
        while shift > 0 {
            shifted.shr1(); shift -= 1;
            q.shl1();
            if r.ge(&shifted) { r = r.sub(&shifted); q.limbs[0] |= 1; }
        }
        q
    }
}

fn bytes_to_bigint(buf: &[u8]) -> BigUint { BigUint::from_bytes(buf) }
fn bigint_to_bytes(val: &BigUint, buf: &mut [u8], len: usize) { val.to_bytes(buf, len); }
fn mod_pow(base: &BigUint, exp: &BigUint, modulus: &BigUint) -> BigUint { BigUint::mod_pow(base, exp, modulus) }
fn mod_inverse_alt(a: &BigUint, m: &BigUint) -> BigUint { BigUint::mod_inverse(a, m) }

pub struct RsaPeripheral {
    pub base: PeripheralBase,
    pub m_value: [u8; 512],
    pub z_value: [u8; 512],
    pub y_value: [u8; 512],
    pub x_value: [u8; 512],
    pub rsa_interrupt: u32,
    pub m_prime: u32,
    pub mult_mode: u32,
    pub mod_exp_mode: u32,
}

impl RsaPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        RsaPeripheral {
            base: PeripheralBase::new(base_addr, name),
            m_value: [0u8; 512],
            z_value: [0u8; 512],
            y_value: [0u8; 512],
            x_value: [0u8; 512],
            rsa_interrupt: 0,
            m_prime: 0,
            mult_mode: 0,
            mod_exp_mode: 0,
        }
    }

    fn sync_from_memory(&mut self) {
        for i in 0..512 {
            self.m_value[i] = self.base.read_uint8(UART_REG1 + i as u32);
            self.z_value[i] = self.base.read_uint8(UART_REG2 + i as u32);
            self.y_value[i] = self.base.read_uint8(UART_REG3 + i as u32);
            self.x_value[i] = self.base.read_uint8(UART_REG4 + i as u32);
        }
    }

    fn sync_to_memory(&mut self) {
        for i in 0..512 {
            self.base.write_uint8(UART_REG1 + i as u32, self.m_value[i]);
            self.base.write_uint8(UART_REG2 + i as u32, self.z_value[i]);
            self.base.write_uint8(UART_REG3 + i as u32, self.y_value[i]);
            self.base.write_uint8(UART_REG4 + i as u32, self.x_value[i]);
        }
    }

    fn do_mod_exp(&mut self) {
        let len = (self.mod_exp_mode + 1) * 64;
        let x = bytes_to_bigint(&self.x_value[..len as usize]);
        let y = bytes_to_bigint(&self.y_value[..len as usize]);
        let m = bytes_to_bigint(&self.m_value[..len as usize]);
        let result = mod_pow(&x, &y, &m);
        bigint_to_bytes(&result, &mut self.z_value, len as usize);
        self.rsa_interrupt = 1;
    }

    fn do_mult(&mut self) {
        let len = ((7 & self.mult_mode) + 1) * 64;
        if self.mult_mode >= 8 {
            let half = len / 2;
            let x = bytes_to_bigint(&self.x_value[..half as usize]);
            let z = bytes_to_bigint(&self.z_value[half as usize..(2 * half) as usize]);
            let result = x.mul_wide(&z);
            bigint_to_bytes(&result, &mut self.z_value, len as usize);
        } else {
            let x = bytes_to_bigint(&self.x_value[..len as usize]);
            let z = bytes_to_bigint(&self.z_value[..len as usize]);
            let m = bytes_to_bigint(&self.m_value[..len as usize]);
            if 1 == self.m_prime {
                let mut pow = BigUint::one();
                for _ in 0..(8 * len) { pow.shl1(); }
                let inv = mod_inverse_alt(&pow, &m);
                let prod = x.mod_mul(&z, &m);
                let result = prod.mod_mul(&inv, &m);
                bigint_to_bytes(&result, &mut self.z_value, len as usize);
            } else {
                let word_count = (len / 4) as usize;
                let mut reg_val = BigUint::zero();
                for t in 0..word_count {
                    let limb_idx = t / 2;
                    let shift = (t % 2) * 32;
                    let x_word = (x.limbs[limb_idx] >> shift) as u32;
                    let low32 = reg_val.limbs[0] as u32;
                    let z_low32 = z.limbs[0] as u32;
                    let signal_direction = (low32.wrapping_add(x_word.wrapping_mul(z_low32))).wrapping_mul(self.m_prime);
                    let mut carry = 0u128;
                    let xw = x_word as u128;
                    let sd = signal_direction as u128;
                    for j in 0..RSA_LIMBS {
                        let zj = z.limbs[j] as u128;
                        let mj = m.limbs[j] as u128;
                        let total = (reg_val.limbs[j] as u128).wrapping_add(xw * zj).wrapping_add(sd * mj).wrapping_add(carry);
                        reg_val.limbs[j] = total as u64;
                        carry = total >> 64;
                    }
                    reg_val.shr32();
                }
                if reg_val.ge(&m) { reg_val = reg_val.sub(&m); }
                bigint_to_bytes(&reg_val, &mut self.z_value, len as usize);
            }
        }
        self.rsa_interrupt = 1;
    }
}

impl MmioPeripheral for RsaPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        match addr & 0xFFF {
            UART_REG5 => self.m_prime,
            UART_REG6 => self.mod_exp_mode,
            UART_REG8 => self.mult_mode,
            UART_REG10 => { if self.rsa_interrupt != 0 { 1 } else { 0 } }
            UART_REG11 => {
                self.m_value = [0u8; 512];
                self.x_value = [0u8; 512];
                self.y_value = [0u8; 512];
                self.z_value = [0u8; 512];
                self.sync_to_memory();
                1
            }
            _ => self.base.read_uint32(addr),
        }
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        self.base.write_uint32(addr, val);
        self.sync_from_memory();
        match addr & 0xFFF {
            UART_REG5 => { self.m_prime = val; }
            UART_REG6 => { self.mod_exp_mode = 7 & val; }
            UART_REG7 => { if 1 & val != 0 { self.do_mod_exp(); } }
            UART_REG8 => { self.mult_mode = 15 & val; }
            UART_REG9 => { if 1 & val != 0 { self.do_mult(); } }
            UART_REG10 => { if 1 & val != 0 { self.rsa_interrupt = 0; } }
            _ => {}
        }
        self.sync_to_memory();
    }

    fn reset(&mut self) {
        self.m_prime = 0;
        self.mod_exp_mode = 0;
        self.mult_mode = 0;
        self.rsa_interrupt = 0;
    }
}
