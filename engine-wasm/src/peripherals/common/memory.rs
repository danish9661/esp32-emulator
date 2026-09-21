use crate::peripherals::types::*;
use core::sync::atomic::{AtomicBool, Ordering};

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u32 = 1 << PAGE_SHIFT;
pub const PAGE_ENTRIES: u32 = 0x100000;
pub const PT_SIZE: u32 = PAGE_ENTRIES * 2;

pub const PTE_TYPE_INVALID: u32 = 0;
pub const PTE_TYPE_RAM: u32 = 1;
pub const PTE_TYPE_MMIO: u32 = 2;

pub static READONLY_OVERRIDE: AtomicBool = AtomicBool::new(false);

pub struct Memory {
    pub data: *mut u8,
    pub len: u32,
    pub base: u32,
}

impl Memory {
    pub fn new(data: *mut u8, len: u32, base: u32) -> Self {
        Memory { data, len, base }
    }

    pub fn base_addr(&self) -> u32 {
        self.base
    }

    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.base && addr < self.base + self.len
    }

    pub fn read_u8(&self, addr: u32) -> u32 {
        let offset = (addr - self.base) as usize;
        unsafe { *self.data.add(offset) as u32 }
    }

    pub fn read_u16(&self, addr: u32) -> u32 {
        let offset = (addr - self.base) as usize;
        unsafe {
            let low = *self.data.add(offset) as u32;
            let high = (*self.data.add(offset + 1) as u32) << 8;
            low | high
        }
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        let offset = (addr - self.base) as usize;
        unsafe {
            let b0 = *self.data.add(offset) as u32;
            let b1 = (*self.data.add(offset + 1) as u32) << 8;
            let b2 = (*self.data.add(offset + 2) as u32) << 16;
            let b3 = (*self.data.add(offset + 3) as u32) << 24;
            b0 | b1 | b2 | b3
        }
    }

    pub fn write_u8(&mut self, addr: u32, val: u32) {
        let offset = (addr - self.base) as usize;
        unsafe { *self.data.add(offset) = (val & 0xFF) as u8 }
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) {
        let offset = (addr - self.base) as usize;
        unsafe {
            *self.data.add(offset) = (val & 0xFF) as u8;
            *self.data.add(offset + 1) = ((val >> 8) & 0xFF) as u8;
        }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let offset = (addr - self.base) as usize;
        unsafe {
            *self.data.add(offset) = (val & 0xFF) as u8;
            *self.data.add(offset + 1) = ((val >> 8) & 0xFF) as u8;
            *self.data.add(offset + 2) = ((val >> 16) & 0xFF) as u8;
            *self.data.add(offset + 3) = ((val >> 24) & 0xFF) as u8;
        }
    }

    pub fn set(&mut self, src: &[u8], dest_addr: u32) {
        let offset = (dest_addr - self.base) as usize;
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), self.data.add(offset), src.len());
        }
    }

    pub fn copy(&mut self, src: &Memory, src_addr: u32, dest_addr: u32, len: u32) {
        let src_offset = (src_addr - src.base) as usize;
        let dest_offset = (dest_addr - self.base) as usize;
        unsafe {
            core::ptr::copy_nonoverlapping(src.data.add(src_offset), self.data.add(dest_offset), len as usize);
        }
    }

    pub fn create_view(&self, base: u32, offset: u32, len: u32) -> Memory {
        let ptr = unsafe { self.data.add(offset as usize) };
        Memory::new(ptr, len, base)
    }

    pub fn remap(&self, base: u32) -> Memory {
        Memory::new(self.data, self.len, base)
    }
}

pub struct ReadonlyMemory {
    pub mem: Memory,
}

impl ReadonlyMemory {
    pub fn new(data: *mut u8, len: u32, base: u32) -> Self {
        ReadonlyMemory { mem: Memory::new(data, len, base) }
    }

    pub fn base_addr(&self) -> u32 {
        self.mem.base_addr()
    }

    pub fn contains(&self, addr: u32) -> bool {
        self.mem.contains(addr)
    }

    pub fn read_u8(&self, addr: u32) -> u32 {
        self.mem.read_u8(addr)
    }

    pub fn read_u16(&self, addr: u32) -> u32 {
        self.mem.read_u16(addr)
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        self.mem.read_u32(addr)
    }

    pub fn write_u8(&mut self, addr: u32, val: u32) {
        if READONLY_OVERRIDE.load(Ordering::Relaxed) {
            return self.mem.write_u8(addr, val);
        }
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) {
        if READONLY_OVERRIDE.load(Ordering::Relaxed) {
            return self.mem.write_u16(addr, val);
        }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        if READONLY_OVERRIDE.load(Ordering::Relaxed) {
            return self.mem.write_u32(addr, val);
        }
    }

    pub fn set(&mut self, src: &[u8], dest_addr: u32) {
        self.mem.set(src, dest_addr)
    }

    pub fn copy(&mut self, src: &Memory, src_addr: u32, dest_addr: u32, len: u32) {
        self.mem.copy(src, src_addr, dest_addr, len)
    }

    pub fn create_view(&self, base: u32, offset: u32, len: u32) -> ReadonlyMemory {
        let ptr = unsafe { self.mem.data.add(offset as usize) };
        ReadonlyMemory::new(ptr, len, base)
    }

    pub fn remap(&self, base: u32) -> Memory {
        self.mem.remap(base)
    }
}

pub struct InvalidMemory {
    pub default_value: u32,
}

impl InvalidMemory {
    pub fn new(default_value: u32) -> Self {
        InvalidMemory { default_value }
    }

    pub fn read_u8(&self) -> u32 {
        0xFF & self.default_value
    }

    pub fn read_u16(&self) -> u32 {
        0xFFFF & self.default_value
    }

    pub fn read_u32(&self) -> u32 {
        0 | self.default_value
    }

    pub fn write_u8(&mut self) {}

    pub fn write_u16(&mut self) {}

    pub fn write_u32(&mut self) {}
}

pub struct MemoryTranslator {
    pub base: *mut Memory,
    pub delta: u32,
}

impl MemoryTranslator {
    pub fn new(base: *mut Memory, delta: u32) -> Self {
        MemoryTranslator { base, delta }
    }

    pub fn read_u8(&self, addr: u32) -> u32 {
        unsafe { (*self.base).read_u8(addr - self.delta) }
    }

    pub fn read_u16(&self, addr: u32) -> u32 {
        unsafe { (*self.base).read_u16(addr - self.delta) }
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        unsafe { (*self.base).read_u32(addr - self.delta) }
    }

    pub fn write_u8(&mut self, addr: u32, val: u32) {
        unsafe { (*self.base).write_u8(addr - self.delta, val) }
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) {
        unsafe { (*self.base).write_u16(addr - self.delta, val) }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        unsafe { (*self.base).write_u32(addr - self.delta, val) }
    }
}

pub struct ReverseMemory {
    pub data: *mut u8,
    pub len: u32,
    pub base: u32,
}

impl ReverseMemory {
    pub fn new(data: *mut u8, len: u32, base: u32) -> Self {
        ReverseMemory { data, len, base }
    }

    pub fn translate_address(&self, addr: u32) -> u32 {
        let offset = addr - self.base;
        self.len - 4 - (0xFFFFFFFC & offset) + (3 & offset)
    }

    pub fn read_u8(&self, addr: u32) -> u32 {
        let idx = self.translate_address(addr) as usize;
        unsafe { *self.data.add(idx) as u32 }
    }

    pub fn read_u16(&self, addr: u32) -> u32 {
        let idx0 = self.translate_address(addr) as usize;
        let idx1 = self.translate_address(addr + 1) as usize;
        unsafe {
            let low = *self.data.add(idx0) as u32;
            let high = (*self.data.add(idx1) as u32) << 8;
            low | high
        }
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        let base = self.translate_address(addr) as usize;
        unsafe {
            let b0 = *self.data.add(base) as u32;
            let b1 = (*self.data.add(base + 1) as u32) << 8;
            let b2 = (*self.data.add(base + 2) as u32) << 16;
            let b3 = (*self.data.add(base + 3) as u32) << 24;
            b0 | b1 | b2 | b3
        }
    }

    pub fn write_u8(&mut self, addr: u32, val: u32) {
        let idx = self.translate_address(addr) as usize;
        unsafe { *self.data.add(idx) = val as u8 }
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) {
        let idx0 = self.translate_address(addr) as usize;
        let idx1 = self.translate_address(addr + 1) as usize;
        unsafe {
            *self.data.add(idx0) = (val & 0xFF) as u8;
            *self.data.add(idx1) = ((val >> 8) & 0xFF) as u8;
        }
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let base = self.translate_address(addr) as usize;
        unsafe {
            *self.data.add(base) = (val & 0xFF) as u8;
            *self.data.add(base + 1) = ((val >> 8) & 0xFF) as u8;
            *self.data.add(base + 2) = ((val >> 16) & 0xFF) as u8;
            *self.data.add(base + 3) = ((val >> 24) & 0xFF) as u8;
        }
    }
}

pub struct MMUMemory {
    pub mem: Memory,
    pub table: *mut u32,
    pub table_len: u32,
    pub table_offset: u32,
}

impl MMUMemory {
    pub fn new(data: *mut u8, len: u32, base: u32) -> Self {
        MMUMemory {
            mem: Memory::new(data, len, base),
            table: core::ptr::null_mut(),
            table_len: 0,
            table_offset: 0,
        }
    }

    pub fn base_addr(&self) -> u32 {
        self.mem.base_addr()
    }

    pub fn contains(&self, addr: u32) -> bool {
        self.mem.contains(addr)
    }

    pub fn set_table(&mut self, table: *mut u32, table_len: u32, offset: u32) {
        self.table = table;
        self.table_len = table_len;
        self.table_offset = offset;
    }

    pub fn map_address(&self, addr: u32) -> u32 {
        let offset = addr - self.base_addr();
        let idx = offset >> 15;
        let entry = if (self.table_offset + idx) < self.table_len {
            unsafe { *self.table.add((self.table_offset + idx) as usize) }
        } else {
            idx
        };
        self.base_addr() + (entry << 15) + (offset & 32767)
    }

    pub fn read_u8(&self, addr: u32) -> u32 {
        let mapped = self.map_address(addr);
        self.mem.read_u8(mapped)
    }

    pub fn read_u16(&self, addr: u32) -> u32 {
        let mapped = self.map_address(addr);
        self.mem.read_u16(mapped)
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        let mapped = self.map_address(addr);
        self.mem.read_u32(mapped)
    }

    pub fn write_u8(&mut self, addr: u32, val: u32) {
        let mapped = self.map_address(addr);
        self.mem.write_u8(mapped, val)
    }

    pub fn write_u16(&mut self, addr: u32, val: u32) {
        let mapped = self.map_address(addr);
        self.mem.write_u16(mapped, val)
    }

    pub fn write_u32(&mut self, addr: u32, val: u32) {
        let mapped = self.map_address(addr);
        self.mem.write_u32(mapped, val)
    }

    pub fn set(&mut self, src: &[u8], dest_addr: u32) {
        self.mem.set(src, dest_addr)
    }

    pub fn copy(&mut self, src: &Memory, src_addr: u32, dest_addr: u32, len: u32) {
        self.mem.copy(src, src_addr, dest_addr, len)
    }

    pub fn create_view(&self, base: u32, offset: u32, len: u32) -> Memory {
        self.mem.create_view(base, offset, len)
    }

    pub fn remap(&self, base: u32) -> Memory {
        self.mem.remap(base)
    }
}

pub struct PageTable {
    pub table: *mut u32,
    pub len: u32,
}

impl PageTable {
    pub fn new(table: *mut u32, len: u32) -> Self {
        PageTable { table, len }
    }

    pub fn page_index(&self, addr: u32) -> u32 {
        addr >> PAGE_SHIFT
    }

    pub fn get_type(&self, addr: u32) -> u32 {
        let i = self.page_index(addr) * 2;
        unsafe { *self.table.add(i as usize) }
    }

    pub fn get_data(&self, addr: u32) -> u32 {
        let i = self.page_index(addr) * 2 + 1;
        unsafe { *self.table.add(i as usize) }
    }

    pub fn set_range(&mut self, base_addr: u32, size: u32, ptype: u32, data: u32) {
        let start = self.page_index(base_addr);
        let end = self.page_index(base_addr + size - 1);
        for i in start..=end {
            unsafe {
                *self.table.add((i * 2) as usize) = ptype;
                *self.table.add((i * 2 + 1) as usize) = data;
            }
        }
    }

    pub fn set_page(&mut self, addr: u32, ptype: u32, data: u32) {
        let i = self.page_index(addr) * 2;
        unsafe {
            *self.table.add(i as usize) = ptype;
            *self.table.add((i + 1) as usize) = data;
        }
    }
}

pub type MmioReadFn = unsafe fn(*mut u8, u32, u32) -> u32;
pub type MmioWriteFn = unsafe fn(*mut u8, u32, u32, u32);
pub type ReadFn = unsafe fn(*mut u8, u32) -> u32;
pub type WriteFn = unsafe fn(*mut u8, u32, u32);

#[derive(Clone, Copy)]
pub struct MMIOHandler {
    pub context: *mut u8,
    pub mmio_read: Option<MmioReadFn>,
    pub mmio_write: Option<MmioWriteFn>,
    pub read_u8: Option<ReadFn>,
    pub read_u16: Option<ReadFn>,
    pub read_u32: Option<ReadFn>,
    pub write_u8: Option<WriteFn>,
    pub write_u16: Option<WriteFn>,
    pub write_u32: Option<WriteFn>,
}

impl MMIOHandler {
    pub const fn empty() -> Self {
        MMIOHandler {
            context: core::ptr::null_mut(),
            mmio_read: None,
            mmio_write: None,
            read_u8: None,
            read_u16: None,
            read_u32: None,
            write_u8: None,
            write_u16: None,
            write_u32: None,
        }
    }
}

pub const MAX_HANDLERS: usize = 256;

pub struct MMIOHandlerRegistry {
    pub handlers: [MMIOHandler; MAX_HANDLERS],
    pub count: u32,
}

impl MMIOHandlerRegistry {
    pub fn new() -> Self {
        MMIOHandlerRegistry {
            handlers: [MMIOHandler::empty(); MAX_HANDLERS],
            count: 0,
        }
    }

    pub fn register(&mut self, handler: MMIOHandler) -> u32 {
        let id = self.count;
        self.handlers[id as usize] = handler;
        self.count += 1;
        id
    }

    pub fn get(&self, id: u32) -> Option<&MMIOHandler> {
        if id < self.count {
            Some(&self.handlers[id as usize])
        } else {
            None
        }
    }

    pub fn read(&self, handler_id: u32, addr: u32, size: u32) -> u32 {
        if handler_id >= self.count {
            return 0;
        }
        let h = &self.handlers[handler_id as usize];
        match size {
            1 => {
                if let Some(f) = h.mmio_read {
                    unsafe { f(h.context, addr, 1) }
                } else if let Some(f) = h.read_u8 {
                    unsafe { f(h.context, addr) }
                } else {
                    0
                }
            }
            2 => {
                if let Some(f) = h.mmio_read {
                    unsafe { f(h.context, addr, 2) }
                } else if let Some(f) = h.read_u16 {
                    unsafe { f(h.context, addr) }
                } else {
                    0
                }
            }
            4 => {
                if let Some(f) = h.mmio_read {
                    unsafe { f(h.context, addr, 4) }
                } else if let Some(f) = h.read_u32 {
                    unsafe { f(h.context, addr) }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, handler_id: u32, addr: u32, val: u32, size: u32) {
        if handler_id >= self.count {
            return;
        }
        let h = &self.handlers[handler_id as usize];
        match size {
            1 => {
                if let Some(f) = h.mmio_write {
                    unsafe { f(h.context, addr, val, 1) }
                } else if let Some(f) = h.write_u8 {
                    unsafe { f(h.context, addr, val) }
                }
            }
            2 => {
                if let Some(f) = h.mmio_write {
                    unsafe { f(h.context, addr, val, 2) }
                } else if let Some(f) = h.write_u16 {
                    unsafe { f(h.context, addr, val) }
                }
            }
            4 => {
                if let Some(f) = h.mmio_write {
                    unsafe { f(h.context, addr, val, 4) }
                } else if let Some(f) = h.write_u32 {
                    unsafe { f(h.context, addr, val) }
                }
            }
            _ => {}
        }
    }
}
