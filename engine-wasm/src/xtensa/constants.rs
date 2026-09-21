// Faithful translation of xtensa-constants.js
// Register indices
pub const LOOP_BEGIN: usize = 0;
pub const LOOP_END: usize = 1;
pub const LOOP_COUNT: usize = 2;
pub const EXC_CAUSE: usize = 3;
pub const PS_REGISTER: usize = 4;
pub const SAR_REGISTER: usize = 12;
pub const LBEG_REGISTER: usize = 16;
pub const WINDOW_START: usize = 17;
pub const INTERRUPT_STATE: usize = 32;
pub const INTERRUPT_ENABLE: usize = 33;
pub const INTERRUPT_CLEAR: usize = 34;
pub const INTERRUPT_SET: usize = 35;
pub const MEM_FAULT_INFO: usize = 72;
pub const CACHE_CONTROL: usize = 73;
pub const ATOM_CTRL: usize = 97;
pub const DDR_REGISTER: usize = 176;
pub const MISC_REGISTER: usize = 177;
pub const MISC1_REGISTER: usize = 178;
pub const EPS_REGISTER: usize = 192;
pub const DEPC_REGISTER: usize = 194;
pub const EPS2_REGISTER: usize = 208;
pub const INT_ENABLE: usize = 226;
pub const INT_CLEAR: usize = 227;
pub const CLOCK_CONFIG: usize = 228;
pub const INT_SET: usize = 230;
pub const INT_LEVEL: usize = 231;
pub const INT_STATUS: usize = 232;
pub const INT_RAW: usize = 233;
pub const CCOUNT_REG: usize = 234;
pub const CCOMPARE_REG: usize = 235;
pub const MISC_CONFIG: usize = 238;
pub const CCOMPARE0_REG: usize = 240;
pub const CCOMPARE1_REG: usize = 241;
pub const CCOMPARE2_REG: usize = 242;
pub const INT_SET_ALIAS: usize = 230;
pub const INT_LEVEL_ALIAS: usize = 231;
pub const INT_STATUS_ALIAS: usize = 232;
pub const INT_RAW_ALIAS: usize = 233;
pub const CCOUNT_ALIAS: usize = 234;
pub const CCOMPARE_ALIAS: usize = 235;
pub const CCOMPARE3_REG: usize = 236;

// Trap causes
pub const TRAP_ILLEGAL_INSTRUCTION: u32 = 0;
pub const TRAP_SYSCALL: u32 = 1;
pub const TRAP_INSTRUCTION_FETCH_ERROR: u32 = 2;
pub const TRAP_LOAD_STORE_ERROR: u32 = 3;
pub const TRAP_LEVEL1_INTERRUPT: u32 = 4;
pub const TRAP_ALLOCA: u32 = 5;
pub const TRAP_INTEGER_DIVIDE_BY_ZERO: u32 = 6;
pub const TRAP_PRIVILEGED: u32 = 8;
pub const TRAP_LOAD_STORE_ALIGNMENT: u32 = 9;
pub const TRAP_INSTR_PIF_DATA_ERROR: u32 = 12;
pub const TRAP_LOAD_STORE_PIF_DATA_ERROR: u32 = 13;
pub const TRAP_INSTR_PIF_ADDR_ERROR: u32 = 14;
pub const TRAP_LOAD_STORE_PIF_ADDR_ERROR: u32 = 15;
pub const TRAP_INST_TLB_MISS: u32 = 16;
pub const TRAP_INST_TLB_MULTI_HIT: u32 = 17;
pub const TRAP_INST_FETCH_PRIVILEGE: u32 = 18;
pub const TRAP_INST_FETCH_PROHIBITED: u32 = 20;
pub const TRAP_LOAD_STORE_TLB_MISS: u32 = 24;
pub const TRAP_LOAD_STORE_TLB_MULTI_HIT: u32 = 25;
pub const TRAP_LOAD_STORE_PRIVILEGE: u32 = 26;
pub const TRAP_LOAD_PROHIBITED: u32 = 28;
pub const TRAP_STORE_PROHIBITED: u32 = 29;
pub const TRAP_COPROCESSORN_DISABLED0: u32 = 32;

// Register offsets for exception/interrupt vectors
pub const STACK_ALIGN_SHIFT: u32 = 0;
pub const REG_OFF_64: u32 = 64;
pub const REG_OFF_128: u32 = 128;
pub const REG_OFF_192: u32 = 192;
pub const REG_OFF_256: u32 = 256;
pub const REG_OFF_320: u32 = 320;
pub const REG_OFF_384: u32 = 384;
pub const REG_OFF_448: u32 = 448;
pub const REG_OFF_512: u32 = 512;
pub const REG_OFF_576: u32 = 576;
pub const REG_OFF_768: u32 = 768;
pub const REG_OFF_832: u32 = 832;
pub const REG_OFF_960: u32 = 960;

// Register type for GDB
pub const REG_TYPE_PC: u32 = 0x0000000;
pub const REG_TYPE_AR: u32 = 0x1000000;
pub const REG_TYPE_SPECIAL: u32 = 0x2000000;
pub const REG_TYPE_USER: u32 = 0x3000000;
pub const REG_TYPE_FP: u32 = 0x4000000;
pub const REG_TYPE_MASK: u32 = 0xff000000;

// Misc constants
pub const PHYSICAL_REG_COUNT: u32 = 64;
pub const CHIP_ID: u32 = 0x306e7458;
pub const CCOMPARE0_INT_BIT: u32 = 6;
pub const CCOMPARE1_INT_BIT: u32 = 15;
pub const CCOMPARE2_INT_BIT: u32 = 16;
pub const INT_ENABLE_MASK_BASE: u32 = 0x20000080;
pub const INT_ENABLE_MASK_2: u32 = 98368;
pub const INT_ENABLE_MASK_3: u32 = 16384;
pub const INT_ENABLE_MASK_4: u32 = 0x50400400;
pub const INST_WIDTH_TABLE: [u32; 16] = [3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4];

// Page table constants (from memory.js)
pub const PAGE_SHIFT: u32 = 12;
pub const PTE_TYPE_INVALID: u32 = 0;
pub const PTE_TYPE_RAM: u32 = 1;
pub const PTE_TYPE_MMIO: u32 = 2;
pub const PTE_TYPE_FLASH: u32 = 3;

// Memory layout (from state.rs) — page table/RAM/flash live above the
// Rust static-data zone (STATIC_ZONE = 4MB headroom for new statics).
pub const SHA_DATA_SIZE: u32 = 128;
pub const SHA_DATA_OFFSET: u32 = 81920;
pub const PAGE_TABLE_OFFSET: u32 = 4194304;
pub const REGION_TABLE_OFFSET: u32 = 12582912;
pub const RAM_DATA_OFFSET: u32 = 12583168;
// Flash mirror placed after the 6MB RAM budget (matches wasm-memory-layout.js FLASH_DATA_OFFSET)
pub const FLASH_DATA_OFFSET: u32 = 18874624;
// MMU table region index in the region table (mmuTableMemory = _memRegions[5]) and
// offset of the app-core table within it (RegionDromSize = 8192)
pub const MMU_TABLE_REGION_ID: u32 = 5;
pub const MMU_APP_TABLE_DELTA: u32 = 8192;
// Invalid MMU entry flag (bit 8, matches JS `if (256 & phyAddr) return this.invalidMem`)
pub const MMU_ENTRY_INVALID: u32 = 0x100;
