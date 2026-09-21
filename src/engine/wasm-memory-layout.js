// Shared memory layout — MUST match engine-wasm/src/xtensa/state.rs and memory.rs
// The entire simulator state lives in one SAB for direct WASM+JS sharing.

export const CORE_STATE_SIZE = 4096;
export const CORES = 20;

export const PAGE_SHIFT = 12;
export const PAGE_SIZE = 1 << PAGE_SHIFT;
export const PAGE_ENTRIES = 1048576;   // 2^20 entries (covers 4GB address space)

export const PAGE_TABLE_SIZE = PAGE_ENTRIES * 8;   // 8,388,608 bytes (2 u32 per entry)
export const SHA_DATA_SIZE = 128;               // SHA peripheral data regs (0x00-0x7F)
export const SHA_DATA_OFFSET = CORE_STATE_SIZE * CORES;

// The Rust `static mut` data zone (register files etc.) lives at the bottom
// of linear memory (~[0x100000, 0x210400] in current builds). All JS-owned
// regions must sit ABOVE it: the linker's static-data placement shifts when
// new statics are added, and statics landing on live PTE entries broke boot
// (AGENTS.md 4d40024). STATIC_ZONE = 4MB headroom for future peripherals.
export const STATIC_ZONE = 4 * 1024 * 1024;

export const PAGE_TABLE_OFFSET = STATIC_ZONE;

export const REGION_TABLE_ENTRIES = 32;
export const REGION_TABLE_SIZE = REGION_TABLE_ENTRIES * 8;  // 256 bytes
export const REGION_TABLE_OFFSET = PAGE_TABLE_OFFSET + PAGE_TABLE_SIZE;

export const RAM_DATA_OFFSET = REGION_TABLE_OFFSET + REGION_TABLE_SIZE;
export const RAM_BUDGET = 6 * 1024 * 1024;   // 6MB for RAM data

// Flash mirror for the Rust flash fast-path (PTE_TYPE_FLASH) — placed right
// after the RAM budget. MUST match constants.rs FLASH_DATA_OFFSET.
export const FLASH_DATA_OFFSET = RAM_DATA_OFFSET + RAM_BUDGET;

// MMU table region index within the region table (mmuTableMemory = _memRegions[5])
export const MMU_TABLE_REGION_ID = 5;

export const TOTAL_SIZE = FLASH_DATA_OFFSET + 4 * 1024 * 1024;

// Native MMIO handler flag — match NATIVE_HANDLER_FLAG in native_mmio.rs
export const NATIVE_HANDLER_FLAG = 0x80000000;
