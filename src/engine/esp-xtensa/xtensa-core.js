import {
  RegisterType,
  ExcCause,
  LbegRegister,
  WindowStart,
  MemFaultInfo,
  AtomCtrl,
  MiscRegister,
  EpsRegister,
  IntEnable,
  IntClear,
  IntSet,
  IntLevel,
  IntStatus,
  CcountReg,
  CcompareReg,
  MiscConfig,
  Ccompare0Reg,
  Ccompare1Reg,
  Ccompare2Reg,
  trapCauseMap,
  pieEnabledFlag,
  regOff768,
  regOff832,
  regOff960,
  PhysicalRegCount,
  Ccompare0IntBit,
  Ccompare1IntBit,
  Ccompare2IntBit,
  IntEnableMaskBase,
  IntEnableMask2,
  IntEnableMask3,
  IntEnableMask4
} from "./xtensa-constants.js";
import { PAGE_SHIFT, PTE_TYPE_RAM, PTE_TYPE_MMIO } from "../../peripherals/common/memory.js";
export class XtensaCore {
  set sarByte(addr) {
    this.userRegisters[13] = 255 & addr;
  }
  set fftBitWidth(addr) {
    this.userRegisters[14] = 255 & addr;
  }
  constructor(esp32, index, name, processorId, gdbRegisters) {
    ((this.esp32 = esp32),
      (this.index = index),
      (this.name = name),
      (this.processorId = processorId),
      (this.gdbRegisters = gdbRegisters),
      (this.physicalRegisters = new Uint32Array(PhysicalRegCount)),
      (this.floatRegisters = new Float32Array(16)),
      (this.floatRegistersUint32 = new Uint32Array(this.floatRegisters.buffer)),
      (this.specialRegisters = new Uint32Array(256)),
      (this.userRegisters = new Uint32Array(237)),
      (this.qRegisters = new Uint8Array(128)),
      (this.qRegistersUint32 = new Uint32Array(this.qRegisters.buffer)),
      (this.qRegistersInt32 = new Int32Array(this.qRegisters.buffer)),
      (this.qRegistersInt16 = new Int16Array(this.qRegisters.buffer)),
      (this.qRegistersUint16 = new Uint16Array(this.qRegisters.buffer)),
      (this.qRegistersInt8 = new Int8Array(this.qRegisters.buffer)),
      (this.qaccHigh = new Uint8Array(20)),
      (this.qaccLow = new Uint8Array(20)),
      (this.accx = 0n),
      
      
      
      
      
      (this.gdbRegisterCount = this.gdbRegisters.length),
      (this.enabled = true),
      (this.idle = false),
      (this.lightSleep = false),
      (this.pendingInterrupts = false),
      (this.opcodeSegment = 0),
      (this._dataPageIdx = -1),
      (this._dataPageType = -1),
      (this._dataPageData = null),
      (this.pageTable = null),
      (this.mmioHandlers = null),
      (this._memRegions = null),
      
      
      (this._cbTraceMemWrite = null),
      
      
      (this._cbWriteWatchpoint = null),
      (this._cbGetCpuTicks = null),
      (this.ccompare0Callback = () => {
        ((this.specialRegisters[IntEnable] |= 1 << Ccompare0IntBit),
          (this.pendingInterrupts = true));
      }),
      (this.ccompare1Callback = () => {
        ((this.specialRegisters[IntEnable] |= 1 << Ccompare1IntBit),
          (this.pendingInterrupts = true));
      }),
      (this.ccompare2Callback = () => {
        ((this.specialRegisters[IntEnable] |= 1 << Ccompare2IntBit),
          (this.pendingInterrupts = true));
      }),
      (this.ccompare0Event = esp32.clocks.cpu.createEvent(
        this.ccompare0Callback,
      )),
      (this.ccompare1Event = esp32.clocks.cpu.createEvent(
        this.ccompare1Callback,
      )),
      (this.ccompare2Event = esp32.clocks.cpu.createEvent(
        this.ccompare2Callback,
      )),
      this.reset());
  }
  attachMemorySystem(pageTable, mmioHandlers, memRegions, callbacks) {
    this.pageTable = pageTable;
    this.mmioHandlers = mmioHandlers;
    this._memRegions = memRegions;
    this._dataPageIdx = -1;
    if (callbacks) {
      this._cbTraceEntry = callbacks.traceEntry ?? null;
      this._cbTraceReturn = callbacks.traceReturn ?? null;
      this._cbTraceMemWrite = callbacks.traceMemWrite ?? null;
      this._cbOnBreak = callbacks.onBreak ?? null;
      this._cbOnUnknownInst = callbacks.onUnknownInst ?? null;
      this._cbWriteWatchpoint = callbacks.writeWatchpoint ?? null;
      this._cbGetCpuTicks = callbacks.getCpuTicks ?? null;
    }
  }
  _updatePageCache(pageIdx) {
    const tbl = this.pageTable.table;
    this._dataPageIdx = pageIdx;
    this._dataPageType = tbl[pageIdx * 2];
    this._dataPageData = tbl[pageIdx * 2 + 1];
  }
  reset() {
    ((this.pendingInterrupts = false),
      (this.enabled = true),
      (this.idle = false),
      (this.lightSleep = false),
      this.physicalRegisters.fill(0),
      this.floatRegisters.fill(0),
      this.specialRegisters.fill(0),
      this.userRegisters.fill(0),

      (this.PC = 0x40000400),
      (this.PS_WOE = 1),
      this.setAR(1, 0x3fffffff),
      (this.specialRegisters[CcompareReg] = this.processorId),
      (this.specialRegisters[AtomCtrl] = 1),
      (this.specialRegisters[IntLevel] = 0x40000000),
      (this.specialRegisters[CcountReg] = 0x5360730),
      (this.nextPC = this.PC));
  }
  set SAR(addr) {
    this.specialRegisters[ExcCause] = addr;
  }
  set PS_INTLEVEL(addr) {
    ((this.specialRegisters[IntSet] =
      (0xfffffff0 & this.specialRegisters[IntSet]) | (15 & addr)),
      this.updateInterrupts());
  }
  get PS_EXCM() {
    return (this.specialRegisters[IntSet] >> 4) & 1;
  }
  set PS_EXCM(addr) {
    ((this.specialRegisters[IntSet] =
      (0xffffffef & this.specialRegisters[IntSet]) | ((1 & addr) << 4)),
      (this.pendingInterrupts = true));
  }
  set PS_UM(addr) {
    this.specialRegisters[IntSet] =
      (0xffffffdf & this.specialRegisters[IntSet]) | ((1 & addr) << 5);
  }
  set PS_OWB(addr) {
    this.specialRegisters[IntSet] =
      (0xfffff0ff & this.specialRegisters[IntSet]) | ((15 & addr) << 8);
  }
  set PS_CALLINC(addr) {
    this.specialRegisters[IntSet] =
      (0xfffcffff & this.specialRegisters[IntSet]) | ((3 & addr) << 16);
  }
  set PS_WOE(addr) {
    this.specialRegisters[IntSet] =
      (0xfffbffff & this.specialRegisters[IntSet]) | ((1 & addr) << 18);
  }
  set ACC(addr) {
    ((this.specialRegisters[WindowStart] =
      255 & Math.floor(addr / 0x100000000)),
      (this.specialRegisters[LbegRegister] = 0 | addr));
  }
  setAR(addr, value) {
    let regIdx = this.specialRegisters[MemFaultInfo] << 2;
    this.physicalRegisters[(regIdx + addr) % PhysicalRegCount] = value;
  }
  vector(addr) {
    return this.specialRegisters[IntLevel] + addr;
  }
  exception(addr) {
    (pieEnabledFlag &&
      console.log(
        `[${this.name}] CPU exception: ${addr} @ 0x${this.PC.toString(16)}`,
      ),
      this.PS_EXCM
        ? ((this.specialRegisters[EpsRegister] = this.PC),
          (this.nextPC = this.vector(regOff960)))
        : this.PS_UM
          ? ((this.specialRegisters[MiscRegister] = this.PC),
            (this.nextPC = this.vector(regOff832)))
          : ((this.specialRegisters[MiscRegister] = this.PC),
            (this.nextPC = this.vector(regOff768))),
      (this.specialRegisters[IntStatus] = addr),
      (this.PS_EXCM = 1));
  }
  get CCOUNT() {
    return this._cbGetCpuTicks ? this._cbGetCpuTicks() >>> 0 : this.esp32.clocks.cpu.ticks >>> 0;
  }
  ccompareUpdate(addr, value, event) {
    let enabled = this.specialRegisters[IntEnable] & (1 << value);
    ((this.specialRegisters[IntEnable] &= ~(1 << value)),
      enabled || event.unschedule());
    let delayTicks =
      addr === this.CCOUNT ? 0xffffffff : (addr - this.CCOUNT) >>> 0;
    event.schedule(delayTicks);
  }
   _readPageTable(addr, size) {
    const pageIdx = addr >>> PAGE_SHIFT;
    if (pageIdx !== this._dataPageIdx) {
      this._updatePageCache(pageIdx);
    }
    if (this._dataPageType === PTE_TYPE_RAM) {
      return this._memRegions[this._dataPageData][`readUint${size}`](addr);
    }
    if (this._dataPageType === PTE_TYPE_MMIO) {
      // If native flag is set, this PTE belongs to a Rust native handler — fall through to mapAddress
      if (this._dataPageData & 0x80000000) {
        return this.esp32.mapAddress(addr, this.index)[`readUint${size}`](addr);
      }
      const handlerId = this._dataPageData;
      return this.mmioHandlers.read(handlerId, addr, size >>> 3);
    }
    return this.esp32.mapAddress(addr, this.index)[`readUint${size}`](addr);
  }
  _writePageTable(addr, val, size) {
    const pageIdx = addr >>> PAGE_SHIFT;
    if (pageIdx !== this._dataPageIdx) {
      this._updatePageCache(pageIdx);
    }
    if (this._dataPageType === PTE_TYPE_RAM) {
      this._memRegions[this._dataPageData][`writeUint${size}`](addr, val);
    } else if (this._dataPageType === PTE_TYPE_MMIO) {
      // If native flag is set, this PTE belongs to a Rust native handler — fall through to mapAddress
      if (this._dataPageData & 0x80000000) {
        this.esp32.mapAddress(addr, this.index)[`writeUint${size}`](addr, val);
        return;
      }
      const handlerId = this._dataPageData;
      this.mmioHandlers.write(handlerId, addr, val, size >>> 3);
    } else {
      this.esp32.mapAddress(addr, this.index)[`writeUint${size}`](addr, val);
    }
  }
  readUint8(addr) {
    if (this.pageTable) return this._readPageTable(addr, 8);
    return this.esp32.mapAddress(addr, this.index).readUint8(addr);
  }
  readUint16(addr) {
    if (this.pageTable) return this._readPageTable(addr, 16);
    return this.esp32.mapAddress(addr, this.index).readUint16(addr);
  }
  readUint32(addr) {
    if (this.pageTable) return this._readPageTable(addr, 32);
    return this.esp32.mapAddress(addr, this.index).readUint32(addr);
  }
  _writeWithTraps(addr, val, size) {
    if (this._cbWriteWatchpoint) {
      this._cbWriteWatchpoint(addr, this);
    } else {
      this.esp32.writeWatchPoints.has(addr) && this.esp32.onBreak?.(this);
    }
    if (this._cbTraceMemWrite) {
      this._cbTraceMemWrite(this.index, this.PC, addr, val, size);
    } else {
      this.esp32.trace.traceMemWrite(this.index, this.PC, addr, val, size);
    }
  }
  writeUint8(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      (this.writeSpecialRegister(MiscConfig, addr),
        this.exception(trapCauseMap.StoreProhibited));
      return;
    }
    this._writeWithTraps(addr, value, 8);
    if (this.pageTable) { this._writePageTable(addr, value, 8); return; }
    this.esp32.mapAddress(addr, this.index).writeUint8(addr, value);
  }
  writeUint16(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      (this.writeSpecialRegister(MiscConfig, addr),
        this.exception(trapCauseMap.StoreProhibited));
      return;
    }
    this._writeWithTraps(addr, value, 16);
    if (this.pageTable) { this._writePageTable(addr, value, 16); return; }
    this.esp32.mapAddress(addr, this.index).writeUint16(addr, value);
  }
  writeUint32(addr, value, regIdx = false) {
    if (regIdx && 0 === addr) {
      (this.writeSpecialRegister(MiscConfig, addr),
        this.exception(trapCauseMap.StoreProhibited),
        false);
      return false;
    }
    this._writeWithTraps(addr, value, 32);
    if (this.pageTable) { this._writePageTable(addr, value, 32); return true; }
    this.esp32.mapAddress(addr, this.index).writeUint32(addr, value);
    return true;
  }
  writeSpecialRegister(addr, value) {
    switch (addr) {
      case IntSet:
        this.pendingInterrupts = true;
        break;
      case Ccompare0Reg:
        this.ccompareUpdate(value, Ccompare0IntBit, this.ccompare0Event);
        break;
      case Ccompare1Reg:
        this.ccompareUpdate(value, Ccompare1IntBit, this.ccompare1Event);
        break;
      case Ccompare2Reg:
        this.ccompareUpdate(value, Ccompare2IntBit, this.ccompare2Event);
        break;
      case IntEnable:
        ((this.specialRegisters[IntEnable] |= value & IntEnableMaskBase),
          (this.pendingInterrupts = true));
        return;
      case IntClear: {
        let regIdx =
          IntEnableMask4 | IntEnableMask3 | IntEnableMask2 | IntEnableMaskBase;
        this.specialRegisters[IntEnable] &= ~(value & regIdx);
        return;
      }
    }
    this.specialRegisters[addr] = value;
  }
  gdbReadRegister(addr) {
    let value = this.gdbRegisters[addr] & RegisterType.Mask,
      regIdx = this.gdbRegisters[addr] & ~RegisterType.Mask;
    switch (value) {
      case RegisterType.PC:
        return this.PC;
      case RegisterType.Special:
        return this.specialRegisters[regIdx];
      case RegisterType.User:
        return this.userRegisters[regIdx];
      case RegisterType.AR:
        return this.physicalRegisters[regIdx];
      case RegisterType.FP:
        return this.floatRegistersUint32[regIdx];
    }
    return (console.warn("Unknown register", addr), 0);
  }
  gdbWriteRegister(addr, value) {
    let regIdx = this.gdbRegisters[addr] & RegisterType.Mask,
      regType = this.gdbRegisters[addr] & ~RegisterType.Mask;
    switch (regIdx) {
      case RegisterType.PC:
        this.PC = value;
        break;
      case RegisterType.Special:
        this.specialRegisters[regType] = value;
        break;
      case RegisterType.User:
        this.userRegisters[regType] = value;
        break;
      case RegisterType.AR:
        this.physicalRegisters[regType] = value;
        break;
      case RegisterType.FP:
        this.floatRegistersUint32[regType] = value;
        break;
      default:
        console.warn("Unknown register", addr);
    }
  }
}
