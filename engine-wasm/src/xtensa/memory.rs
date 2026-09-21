use super::constants::*;
use super::state::CoreState;

use crate::native_mmio::{NATIVE_HANDLER_FLAG, native_mmio_read, native_mmio_write};
extern "C" {
    fn mmio_read(handler_id: u32, addr: u32, size: u32) -> u32;
    fn mmio_write(handler_id: u32, addr: u32, val: u32, size: u32);
    fn map_address(core_idx: u32, addr: u32) -> u32;
    fn map_read(core_idx: u32, addr: u32, size: u32) -> u32;
    fn map_write(core_idx: u32, addr: u32, val: u32, size: u32);
    fn js_flash_write_override() -> u32;
}

// Flash fast-path state (set by native_flash_init from JS)
static mut FLASH_OFF: u32 = 0;
static mut MMU_REGION_ID: u32 = MMU_TABLE_REGION_ID;

// DMA descriptor/buffer access for native peripherals (JS parity:
// this.cpu.mapAddress(addr, coreIdx) — the page-cached JS map path used
// by the JS DmaDescriptorChain in i2c-i2s.js). core 0, like the JS
// peripherals pass coreIdx 0 to mapAddress. Now resolves the page table
// natively (RAM / flash / MMIO / JS fallback) instead of the map_read FFI.
// Size convention: BITS (8/16/32) throughout — the RAM branch matches on
// 8/16, the MMIO branch converts via size >> 3, and flash/map pass the
// bit size through to the JS readUint8/16/32 handlers.
pub fn dma_read_u32(addr: u32) -> u32 {
    dma_read(addr, 32)
}
pub fn dma_write_u32(addr: u32, val: u32) {
    dma_write(addr, val, 32);
}
pub fn dma_read_u8(addr: u32) -> u8 {
    dma_read(addr, 8) as u8
}
pub fn dma_write_u8(addr: u32, val: u32) {
    dma_write(addr, val, 8);
}

fn dma_read(addr: u32, size: u32) -> u32 {
    let (typ, data) = page_table_entry(addr >> PAGE_SHIFT);
    if typ == PTE_TYPE_RAM {
        let a = ram_addr(data, addr);
        unsafe {
            match size {
                8 => mem_read8(a) as u32,
                16 => mem_read16(a) as u32,
                _ => mem_read32(a),
            }
        }
    } else if typ == PTE_TYPE_FLASH {
        unsafe { flash_read(CoreState::from_index(0), addr, data, size) }
    } else if typ == PTE_TYPE_MMIO {
        let handler_id = data;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, size >> 3)
        } else {
            unsafe { mmio_read(handler_id, addr, size >> 3) }
        }
    } else {
        unsafe { map_read(0, addr, size) }
    }
}

fn dma_write(addr: u32, val: u32, size: u32) {
    let (typ, data) = page_table_entry(addr >> PAGE_SHIFT);
    if typ == PTE_TYPE_RAM {
        let a = ram_addr(data, addr);
        unsafe {
            match size {
                8 => mem_write8(a, val as u8),
                16 => mem_write16(a, val as u16),
                _ => mem_write32(a, val),
            }
        }
    } else if typ == PTE_TYPE_FLASH {
        unsafe {
            if js_flash_write_override() != 0 {
                map_write(0, addr, val, size);
            }
        }
    } else if typ == PTE_TYPE_MMIO {
        let handler_id = data;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_write(handler_id & !NATIVE_HANDLER_FLAG, addr, val, size >> 3);
        } else {
            unsafe { mmio_write(handler_id, addr, val, size >> 3) }
        }
    } else {
        unsafe { map_write(0, addr, val, size) }
    }
}

pub fn init_flash(flash_off: u32, mmu_region_id: u32) {
    unsafe {
        FLASH_OFF = flash_off;
        MMU_REGION_ID = mmu_region_id;
    }
}

// Boot-safe raw flash-mirror read (no MMU/PTE dependency): the .flash.text
// VMA window 0x400D0020.. maps linearly into the flash mirror base
// (FLASH_OFF + (vma - 0x400D0020)). Used by the bt_hook_scan signature
// scanner, which runs from the first boot-ROM step — long before the MMU
// tables are seeded (dma_read_u32 on flash falls to the JS map_read FFI =
// 0xFFFFFFFF noise there = scan matches garbage = h34 lesson).
// CORRECTNESS (run265r, measured): the flash FILE is a bootloader image
// (segments at file offsets), NOT a 1:1 VMA window — the app .flash.text
// seg3 (VMA 0x400D0020) lives at file offset 0x40020. So the mirror word at
// FLASH_OFF+0 is 0xFFFFFFFF (erased), and VMA - 0x400D0020 math reads the
// WRONG bytes (off by 0x40020). The scanner must use the SEG3 file offset:
// FLASH_SEG3_OFF + (vma - 0x400D0020). Set once by native_flash_init_seg3.
static mut FLASH_SEG3_OFF: u32 = 0;
pub fn init_flash_seg3(seg3_file_off: u32) {
    unsafe { FLASH_SEG3_OFF = seg3_file_off; }
}
pub fn flash_mirror_read_u32(vma: u32) -> u32 {
    unsafe {
        if vma < 0x400D0020 {
            return 0xFFFFFFFF;
        }
        let off = FLASH_SEG3_OFF.wrapping_add(vma.wrapping_sub(0x400D0020));
        if off > 4 * 1024 * 1024 - 4 {
            return 0xFFFFFFFF;
        }
        mem_read32(FLASH_OFF.wrapping_add(off))
    }
}

// Raw memory access helpers for WASM linear memory
unsafe fn mem_read8(addr: u32) -> u8 {
    *(addr as *const u8)
}
unsafe fn mem_read16(addr: u32) -> u16 {
    let p = addr as *const u8;
    (p.read() as u16) | ((p.add(1).read() as u16) << 8)
}
unsafe fn mem_read32(addr: u32) -> u32 {
    (addr as *const u32).read_unaligned()
}
unsafe fn mem_write8(addr: u32, val: u8) {
    *(addr as *mut u8) = val;
}
unsafe fn mem_write16(addr: u32, val: u16) {
    let p = addr as *mut u8;
    p.write(val as u8);
    p.add(1).write((val >> 8) as u8);
}
unsafe fn mem_write32(addr: u32, val: u32) {
    (addr as *mut u32).write_unaligned(val);
}

// Region table access: region_id → (base_addr, wasm_offset)
// Located at REGION_TABLE_OFFSET in linear memory
unsafe fn region_info(region_id: u32) -> (u32, u32) {
    let off = (REGION_TABLE_OFFSET + region_id * 8) as usize;
    let base = *(off as *const u32);
    let wasm_off = *((off + 4) as *const u32);
    (base, wasm_off)
}

// Compute WASM linear memory address for RAM access
fn ram_addr(region_id: u32, addr: u32) -> u32 {
    unsafe {
        let (base, wasm_off) = region_info(region_id);
        RAM_DATA_OFFSET + wasm_off + addr.wrapping_sub(base)
    }
}

// Page table access (page table is at PAGE_TABLE_OFFSET in linear memory)
// Each entry: [type_flags, data_or_handler_id]
fn page_table_entry(page_idx: u32) -> (u32, u32) {
    unsafe {
        let off = (PAGE_TABLE_OFFSET + page_idx * 8) as usize;
        let typ = *(off as *const u32);
        let data = *((off + 4) as *const u32);
        (typ, data)
    }
}

// Flash fast-path: MMU table entries live in the mmuTableMemory RAM region
// (region table entry MMU_REGION_ID, base 0x3FF10000). Core 0 uses the PRO
// table at offset 0, core 1 the APP table at +MMU_APP_TABLE_DELTA — matches
// JS mapAddress(cpuVal, coreIdx) selecting mmuTablePro/mmuTableApp.
fn mmu_entry(core: &CoreState, mmu_idx: u32) -> u32 {
    unsafe {
        let (_, wasm_off) = region_info(MMU_REGION_ID);
        let table_base = RAM_DATA_OFFSET + wasm_off + if core.index == 0 { 0 } else { MMU_APP_TABLE_DELTA };
        mem_read32(table_base + (mmu_idx & (MMU_APP_TABLE_DELTA - 1)) * 4)
    }
}

// Flash MMU windows map via `entry << 16 | (addr & 0xFFFF)` (64KB chunks).
// Returns u32::MAX when the entry is invalid (bit 8 set) — the JS path returns
// this.invalidMem (0xFF/0xFFFF/0xFFFFFFFF reads) in that case.
fn flash_linear(core: &CoreState, addr: u32, mmu_idx: u32) -> u32 {
    let entry = mmu_entry(core, mmu_idx);
    if entry & MMU_ENTRY_INVALID != 0 {
        return u32::MAX;
    }
    unsafe { FLASH_OFF + (entry << 16) + (addr & 0xFFFF) }
}

fn flash_read(core: &CoreState, addr: u32, mmu_idx: u32, size: u32) -> u32 {
    let a = flash_linear(core, addr, mmu_idx);
    if a == u32::MAX {
        return match size {
            8 => 0xFF,
            16 => 0xFFFF,
            _ => 0xFFFFFFFF,
        };
    }
    unsafe {
        match size {
            8 => mem_read8(a) as u32,
            16 => mem_read16(a) as u32,
            _ => mem_read32(a),
        }
    }
}

// _updatePageCache (faithful)
pub fn update_page_cache(core: &mut CoreState, page_idx: u32) {
    let (typ, data) = page_table_entry(page_idx);
    core.data_page_idx = page_idx as i32;
    core.data_page_type = typ as i32;
    core.data_page_data = data as i32;
}

// _readPageTable (faithful)
pub fn read_page_table(core: &mut CoreState, addr: u32, size: u32) -> u32 {
    let page_idx = addr >> PAGE_SHIFT;
    if page_idx != core.data_page_idx as u32 {
        update_page_cache(core, page_idx);
    }
    if crate::native_mmio::mmio_read_debug_enabled()
        && core.pc >= 0x4000c040
        && core.pc < 0x4000c100
        && addr < 0x3ff00000
    {
        use core::fmt::Write as _;
        struct DbBuf([u8; 128]);
        impl core::fmt::Write for DbBuf {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let mut i = 0;
                for b in s.bytes() {
                    if i >= self.0.len() { return Ok(()); }
                    self.0[i] = b;
                    i += 1;
                }
                Ok(())
            }
        }
        let mut db = DbBuf([0u8; 128]);
        let _ = core::write!(&mut db, "R{:08x}.{:08x}", core.pc, addr);
        unsafe { crate::js_log_str(db.0.as_ptr() as u32, 128) };
    }
    if core.data_page_type as u32 == PTE_TYPE_RAM {
        let a = ram_addr(core.data_page_data as u32, addr);
        let v = unsafe {
            match size {
                8 => mem_read8(a) as u32,
                16 => mem_read16(a) as u32,
                _ => mem_read32(a),
            }
        };
        if (addr >= 0x3ffc7a00 && addr <= 0x3ffc7d00)
            || (addr >= 0x3ffb6c00 && addr <= 0x3ffb6e00)
            || (addr >= 0x3ffc88c0 && addr <= 0x3ffc8900)
            || (addr >= 0x3ffc8830 && addr <= 0x3ffc8860)
            || (addr >= 0x3ffc70f0 && addr <= 0x3ffc7130)
            || (addr >= 0x3ffc8800 && addr <= 0x3ffc8900)
        {
            crate::xtensa::exports::ring_read(core.pc, addr, v);
        }
        if unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT } > 0
            && ((addr >= 0x3ffd1480 && addr <= 0x3ffd1500)
                || (addr >= 0x3ffc9090 && addr <= 0x3ffc9098)
                || (addr >= 0x3ffc8830 && addr <= 0x3ffc8870)
                || (addr >= 0x3ffc88c0 && addr <= 0x3ffc88f0)
                || (addr >= 0x3ffafd6c && addr <= 0x3ffafdcc)
                || (addr >= 0x3ffc7100 && addr <= 0x3ffc7120)
                || (addr >= 0x3ffc7a10 && addr <= 0x3ffc7a30)
                // TEMP-DIAG-KEPKT (revert): watch LL reads of the H2C KE packet
                || (addr >= 0x3ffce980 && addr <= 0x3ffcea40))
        {
            unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT -= 1; }
            crate::xtensa::exports::bt_vhci_log(core.pc, addr, v, 0);
        }
        v
    } else if core.data_page_type as u32 == PTE_TYPE_MMIO {
        let handler_id = core.data_page_data as u32;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, size >> 3)
        } else {
            unsafe { mmio_read(handler_id, addr, size >> 3) }
        }
    } else if core.data_page_type as u32 == PTE_TYPE_FLASH {
        flash_read(core, addr, core.data_page_data as u32, size)
    } else {
        unsafe { map_read(core.index, addr, size) }
    }
}

// _writePageTable (faithful)
pub fn write_page_table(core: &mut CoreState, addr: u32, val: u32, size: u32) {
    if unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT } > 0
        && addr >= 0x3ffa0000 && addr <= 0x3ffdffff
    {
        unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT -= 1; }
        crate::xtensa::exports::bt_vhci_log(core.pc, addr, val, 1);
    }
    if unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT } > 0
        && ((addr >= 0x3ffd1480 && addr <= 0x3ffd1500)
            || (addr >= 0x3ffc9090 && addr <= 0x3ffc9098)
            || (addr >= 0x3ffc8830 && addr <= 0x3ffc8870)
            || (addr >= 0x3ffc88c0 && addr <= 0x3ffc88f0)
            || (addr >= 0x3ffafd6c && addr <= 0x3ffafdcc)
            || (addr >= 0x3ffc7100 && addr <= 0x3ffc7120)
            || (addr >= 0x3ffc7a10 && addr <= 0x3ffc7a30))
    {
        unsafe { crate::native_mmio::BT_VHCI_TRACE_LEFT -= 1; }
        crate::xtensa::exports::bt_vhci_log(core.pc, addr, val, 1);
    }
    if addr >= 0x3ffc70e0 && addr <= 0x3ffc7130 {
        crate::xtensa::exports::ring_write(core.pc, addr, val);
    } else if addr >= 0x3ffa0000 && addr <= 0x3ffdffff && val == 0x0301040e {
        crate::xtensa::exports::bt_vhci_log(core.pc, addr, val, 1);
    } else if addr >= 0x3ffc85e0 && addr <= 0x3ffc8740 {
        crate::xtensa::exports::ring_write(core.pc, addr, val);
    } else if addr >= 0x3ffc3a40 && addr <= 0x3ffc3b00 {
        crate::xtensa::exports::ring_write(core.pc, addr, val);
    } else if addr >= 0x3ffc7a00 && addr <= 0x3ffc7d00 {
        crate::xtensa::exports::ring_write(core.pc, addr, val);
    }
    let page_idx = addr >> PAGE_SHIFT;
    if page_idx != core.data_page_idx as u32 {
        update_page_cache(core, page_idx);
    }
    if core.data_page_type as u32 == PTE_TYPE_RAM {
        let a = ram_addr(core.data_page_data as u32, addr);
        unsafe {
            match size {
                8 => mem_write8(a, val as u8),
                16 => mem_write16(a, val as u16),
                _ => mem_write32(a, val),
            }
        }
    } else if core.data_page_type as u32 == PTE_TYPE_MMIO {
        let handler_id = core.data_page_data as u32;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_write(handler_id & !NATIVE_HANDLER_FLAG, addr, val, size >> 3);
        } else {
            unsafe { mmio_write(handler_id, addr, val, size >> 3) }
        }
    } else if core.data_page_type as u32 == PTE_TYPE_FLASH {
        // Flash-cache windows are read-only from the CPU. Drop writes
        // (ReadonlyMemory parity) unless GDB flash writes are active —
        // then route through the JS map_write FFI, which honors the
        // ReadonlyMemory.override flag set by gdb-session.js M commands.
        unsafe {
            if js_flash_write_override() != 0 {
                map_write(core.index, addr, val, size);
            }
        }
    } else {
        unsafe { map_write(core.index, addr, val, size) }
    }
}

// _getOpcodeMemory (faithful) — stores type and handler_id/region_id in separate opcode cache
pub fn get_opcode_memory(core: &mut CoreState, addr: u32) {
    let page_idx = addr >> PAGE_SHIFT;
    let (typ, data) = page_table_entry(page_idx);
    core.opcode_page_idx = page_idx as i32;
    core.opcode_page_type = typ as i32;
    core.opcode_page_data = data as i32;
}

// Opcode memory read functions (used by runInstruction)
// These use the cached page info from the last get_opcode_memory call
pub fn read_opcode_u16(core: &CoreState, addr: u32) -> u32 {
    if core.opcode_page_type as u32 == PTE_TYPE_RAM {
        let a = ram_addr(core.opcode_page_data as u32, addr);
        unsafe { mem_read16(a) as u32 }
    } else if core.opcode_page_type as u32 == PTE_TYPE_MMIO {
        let handler_id = core.opcode_page_data as u32;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, 2)
        } else {
            unsafe { mmio_read(handler_id, addr, 2) }
        }
    } else if core.opcode_page_type as u32 == PTE_TYPE_FLASH {
        flash_read(core, addr, core.opcode_page_data as u32, 16)
    } else {
        unsafe { map_read(core.index, addr, 16) }
    }
}

pub fn read_opcode_u32(core: &CoreState, addr: u32) -> u32 {
    if core.opcode_page_type as u32 == PTE_TYPE_RAM {
        let a = ram_addr(core.opcode_page_data as u32, addr);
        unsafe { mem_read32(a) }
    } else if core.opcode_page_type as u32 == PTE_TYPE_MMIO {
        let handler_id = core.opcode_page_data as u32;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, 4)
        } else {
            unsafe { mmio_read(handler_id, addr, 4) }
        }
    } else if core.opcode_page_type as u32 == PTE_TYPE_FLASH {
        flash_read(core, addr, core.opcode_page_data as u32, 32)
    } else {
        unsafe { map_read(core.index, addr, 32) }
    }
}

pub fn read_opcode_u8(core: &CoreState, addr: u32) -> u32 {
    // The +2 byte of a 3-byte opcode can land in the NEXT 4KB page. The
    // opcode page cache is keyed to the fetch page, so re-resolve the page
    // table entry when the pages differ (JS mapAddress re-resolves per
    // access; the cached data here would select the wrong MMU entry, e.g.
    // 0x400E vs 0x400F flash windows).
    if (addr >> PAGE_SHIFT) != core.opcode_page_idx as u32 {
        let (typ, data) = page_table_entry(addr >> PAGE_SHIFT);
        return match typ {
            PTE_TYPE_RAM => {
                let a = ram_addr(data, addr);
                unsafe { mem_read8(a) as u32 }
            }
            PTE_TYPE_MMIO => {
                let handler_id = data;
                if handler_id & NATIVE_HANDLER_FLAG != 0 {
                    native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, 1)
                } else {
                    unsafe { mmio_read(handler_id, addr, 1) }
                }
            }
            PTE_TYPE_FLASH => flash_read(core, addr, data, 8),
            _ => unsafe { map_read(core.index, addr, 8) },
        };
    }
    if core.opcode_page_type as u32 == PTE_TYPE_RAM {
        let a = ram_addr(core.opcode_page_data as u32, addr);
        unsafe { mem_read8(a) as u32 }
    } else if core.opcode_page_type as u32 == PTE_TYPE_MMIO {
        let handler_id = core.opcode_page_data as u32;
        if handler_id & NATIVE_HANDLER_FLAG != 0 {
            native_mmio_read(handler_id & !NATIVE_HANDLER_FLAG, addr, 1)
        } else {
            unsafe { mmio_read(handler_id, addr, 1) }
        }
    } else if core.opcode_page_type as u32 == PTE_TYPE_FLASH {
        flash_read(core, addr, core.opcode_page_data as u32, 8)
    } else {
        unsafe { map_read(core.index, addr, 8) }
    }
}
