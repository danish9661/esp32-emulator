// Translated from: src/peripherals/common/peripheral.js
// Do NOT modify the logic — match the JS line-for-line

use crate::peripherals::types::*;
use core::marker::PhantomData;

// ============================================================
// RegisterField — helper for bitfield register access
// JS: class RegisterField { constructor(cpuVal) { ... } }
// ============================================================

/// Wraps a raw pointer to a u32 within a PeripheralBase memory buffer.
/// Becomes invalid if the owning PeripheralBase is moved.
pub struct RegisterField {
    ptr: *mut u32,
}

impl RegisterField {
    pub fn new(ptr: *mut u32) -> Self {
        RegisterField { ptr }
    }

    // read() — return this.memory[0]
    pub fn read(&self) -> u32 {
        unsafe { *self.ptr }
    }

    // write(cpuVal) — this.memory[0] = cpuVal
    pub fn write(&self, val: u32) {
        unsafe { *self.ptr = val; }
    }

    // bit(cpuVal) — !!(this.memory[0] & cpuVal)
    pub fn bit(&self, bit: u32) -> bool {
        unsafe { (*self.ptr & bit) != 0 }
    }

    // setBits(cpuVal) — this.memory[0] |= cpuVal
    pub fn set_bits(&self, bits: u32) {
        unsafe { *self.ptr |= bits; }
    }

    // clearBits(cpuVal) — this.memory[0] &= ~cpuVal
    pub fn clear_bits(&self, bits: u32) {
        unsafe { *self.ptr &= !bits; }
    }

    // field(cpuVal, tmpVal) — (this.memory[0] >> cpuVal) & tmpVal
    pub fn field(&self, shift: u32, mask: u32) -> u32 {
        unsafe { (*self.ptr >> shift) & mask }
    }

    // setField(cpuVal, tmpVal, idxVal)
    // let ClockEvent = (this.memory[0] & ~tmpVal) | ((idxVal & tmpVal) << cpuVal)
    // return ((this.memory[0] = ClockEvent), ClockEvent)
    pub fn set_field(&self, shift: u32, mask: u32, val: u32) -> u32 {
        unsafe {
            let result = (*self.ptr & !mask) | ((val & mask) << shift);
            *self.ptr = result;
            result
        }
    }
}

// ============================================================
// FieldDesc — describes a bit field within a register
// JS uses plain objects { reg, shift, mask }
// ============================================================

#[derive(Clone, Copy)]
pub struct FieldDesc {
    pub reg: u32,
    pub shift: u32,
    pub mask: u32,
}

impl FieldDesc {
    // JS createFieldDescriptor(reg, shift, width) → { reg, shift, mask: (1 << width) - 1 }
    pub const fn new(reg: u32, shift: u32, width: u32) -> Self {
        FieldDesc { reg, shift, mask: (1u32 << width) - 1 }
    }
}

// ============================================================
// PeripheralBase — base class for all peripherals
// JS: class PeripheralBase { constructor(cpu, baseAddr, name) { ... } }
// ============================================================

pub struct PeripheralBase {
    pub _cpu: PhantomData<*mut ()>,
    pub base_addr: u32,
    pub name: &'static str,
    pub memory: [u8; 4096],
}

impl PeripheralBase {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        PeripheralBase {
            _cpu: PhantomData,
            base_addr,
            name,
            memory: [0u8; 4096],
        }
    }

    // readUint8(cpuVal) — this.memory[cpuVal - this.baseAddr]
    pub fn read_uint8(&self, addr: u32) -> u8 {
        let off = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if off < self.memory.len() { self.memory[off] } else { 0 }
    }

    // readUint16(cpuVal) — this.memoryView.getUint16(cpuVal - this.baseAddr, true)
    pub fn read_uint16(&self, addr: u32) -> u16 {
        let offset = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if offset + 2 <= self.memory.len() {
            u16::from_le_bytes(self.memory[offset..offset + 2].try_into().unwrap())
        } else { 0 }
    }

    // readUint32(cpuVal) — this.memoryView.getUint32(cpuVal - this.baseAddr, true)
    pub fn read_uint32(&self, addr: u32) -> u32 {
        let offset = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if offset + 4 > self.memory.len() { return 0; }
        u32::from_le_bytes(self.memory[offset..offset + 4].try_into().unwrap())
    }

    // writeUint8(cpuVal, tmpVal) — this.memory[cpuVal - this.baseAddr] = tmpVal
    pub fn write_uint8(&mut self, addr: u32, val: u8) {
        let off = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if off < self.memory.len() { self.memory[off] = val; }
    }

    // writeUint16(cpuVal, tmpVal) — this.memoryView.setUint16(cpuVal - this.baseAddr, tmpVal, true)
    pub fn write_uint16(&mut self, addr: u32, val: u16) {
        let offset = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if offset + 2 <= self.memory.len() {
            self.memory[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        }
    }

    // writeUint32(cpuVal, tmpVal) — this.memoryView.setUint32(cpuVal - this.baseAddr, tmpVal, true)
    pub fn write_uint32(&mut self, addr: u32, val: u32) {
        let offset = (addr.wrapping_sub(self.base_addr) & 0xFFF) as usize;
        if offset + 4 <= self.memory.len() {
            self.memory[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        }
    }

    // zeroMemory() — this.memory.fill(0)
    pub fn zero_memory(&mut self) {
        self.memory.fill(0);
    }

    // reset() — {} (empty)
    pub fn reset(&mut self) {}

    // readRegister(cpuVal) — this.memoryView.getUint32(cpuVal, true)
    // Note: cpuVal is a byte offset (NOT addr - baseAddr)
    pub fn read_register(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        u32::from_le_bytes(self.memory[offset..offset + 4].try_into().unwrap())
    }

    // writeRegister(cpuVal, tmpVal) — this.memoryView.setUint32(cpuVal, tmpVal, true)
    pub fn write_register(&mut self, offset: u32, val: u32) {
        let offset = offset as usize;
        self.memory[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    }

    // readField(cpuVal) — (this.readRegister(cpuVal.reg) >> cpuVal.shift) & cpuVal.mask
    pub fn read_field(&self, field: &FieldDesc) -> u32 {
        (self.read_register(field.reg) >> field.shift) & field.mask
    }

    // writeField(cpuVal, tmpVal)
    // let idxVal = this.readRegister(cpuVal.reg)
    // let ClockEvent = cpuVal.mask << cpuVal.shift
    // let SimulationClock = (idxVal & ~ClockEvent) | ((tmpVal << cpuVal.shift) & ClockEvent)
    // this.writeRegister(cpuVal.reg, SimulationClock)
    pub fn write_field(&mut self, field: &FieldDesc, val: u32) {
        let current = self.read_register(field.reg);
        let shifted_mask = field.mask << field.shift;
        let new_val = (current & !shifted_mask) | ((val << field.shift) & shifted_mask);
        self.write_register(field.reg, new_val);
    }

    // writeFieldOpt(cpuVal, tmpVal) — cpuVal && this.writeField(cpuVal, tmpVal)
    pub fn write_field_opt(&mut self, field: Option<&FieldDesc>, val: u32) {
        if let Some(f) = field {
            self.write_field(f, val);
        }
    }

    // setRegisterBits(cpuVal, tmpVal)
    // let idxVal = this.readRegister(cpuVal) | tmpVal
    // return (this.writeRegister(cpuVal, idxVal), idxVal)
    pub fn set_register_bits(&mut self, offset: u32, bits: u32) -> u32 {
        let val = self.read_register(offset) | bits;
        self.write_register(offset, val);
        val
    }

    // clearRegisterBits(cpuVal, tmpVal)
    // let idxVal = this.readRegister(cpuVal) & ~tmpVal
    // return (this.writeRegister(cpuVal, idxVal), idxVal)
    pub fn clear_register_bits(&mut self, offset: u32, bits: u32) -> u32 {
        let val = self.read_register(offset) & !bits;
        self.write_register(offset, val);
        val
    }

    // register(cpuVal)
    // return new RegisterField(new Uint32Array(this.memory.buffer, cpuVal, 1))
    pub fn register(&mut self, offset: u32) -> RegisterField {
        let ptr = unsafe { self.memory.as_mut_ptr().add(offset as usize) as *mut u32 };
        RegisterField::new(ptr)
    }

    // dumpState(cpuVal) — cpuVal.writeBytes(this.memory)
    pub fn dump_state(&self, dest: &mut [u8; 4096]) {
        *dest = self.memory;
    }

    // restoreState(cpuVal) — cpuVal.readBytes(this.memory)
    pub fn restore_state(&mut self, src: &[u8; 4096]) {
        self.memory = *src;
    }
}

// ============================================================
// EmptyPeripheral — extends PeripheralBase
// JS: class EmptyPeripheral extends PeripheralBase {}
// ============================================================

pub struct EmptyPeripheral(pub PeripheralBase);

impl EmptyPeripheral {
    pub fn new(base_addr: u32, name: &'static str) -> Self {
        EmptyPeripheral(PeripheralBase::new(base_addr, name))
    }
}

impl MmioPeripheral for EmptyPeripheral {
    fn read_u32(&mut self, _ctx: &mut CpuContext, addr: u32) -> u32 {
        self.0.read_uint32(addr)
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, addr: u32, val: u32) {
        self.0.write_uint32(addr, val);
    }

    fn reset(&mut self) {
        self.0.reset();
    }
}
