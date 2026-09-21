// Peripheral base classes — extracted from index.js module 50722

export class RegisterField {
  constructor(memory) {
    this.memory = memory;
  }
  read() {
    return this.memory[0];
  }
  write(value) {
    this.memory[0] = value;
  }
  bit(mask) {
    return !!(this.memory[0] & mask);
  }
  setBits(mask) {
    this.memory[0] |= mask;
  }
  clearBits(mask) {
    this.memory[0] &= ~mask;
  }
  field(shift, mask) {
    return (this.memory[0] >> shift) & mask;
  }
  setField(shift, mask, value) {
    const next = (this.memory[0] & ~mask) | ((value & mask) << shift);
    this.memory[0] = next;
    return next;
  }
}

export class PeripheralBase {
  constructor(cpu, baseAddr, name) {
    this.cpu = cpu;
    this.baseAddr = baseAddr;
    this.name = name;
    this.memory = new Uint8Array(new SharedArrayBuffer(4096));
    this.memoryView = new DataView(this.memory.buffer);
    this.registers = [];
  }
  readUint8(addr) {
    if (PeripheralBase.debug)
      console.log(`"${this.name}".readUint8 ${(addr - this.baseAddr).toString(16)}`);
    return this.memory[addr - this.baseAddr];
  }
  readUint16(addr) {
    if (PeripheralBase.debug)
      console.log(`"${this.name}".readUint16 ${(addr - this.baseAddr).toString(16)}`);
    return this.memoryView.getUint16(addr - this.baseAddr, true);
  }
  readUint32(addr) {
    if (PeripheralBase.debug)
      console.log(`"${this.name}".readUint32 ${(addr - this.baseAddr).toString(16)}`);
    return this.memoryView.getUint32(addr - this.baseAddr, true);
  }
  writeUint8(addr, value) {
    if (PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint8 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`,
      );
    this.memory[addr - this.baseAddr] = value;
  }
  writeUint16(addr, value) {
    if (PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint16 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`,
      );
    this.memoryView.setUint16(addr - this.baseAddr, value, true);
  }
  writeUint32(addr, value) {
    if (PeripheralBase.debug)
      console.log(
        `"${this.name}".writeUint32 ${(addr - this.baseAddr).toString(16)} ${value.toString(16)}`,
      );
    this.memoryView.setUint32(addr - this.baseAddr, value, true);
  }
  zeroMemory() {
    this.memory.fill(0);
  }
  reset() {}
  readRegister(offset) {
    return this.memoryView.getUint32(offset, true);
  }
  writeRegister(offset, value) {
    this.memoryView.setUint32(offset, value, true);
  }
  readField(field) {
    return (this.readRegister(field.reg) >> field.shift) & field.mask;
  }
  writeField(field, value) {
    const current = this.readRegister(field.reg);
    const masked = field.mask << field.shift;
    const next = (current & ~masked) | ((value << field.shift) & masked);
    this.writeRegister(field.reg, next);
  }
  writeFieldOpt(field, value) {
    if (field) this.writeField(field, value);
  }
  setRegisterBits(offset, bits) {
    const next = this.readRegister(offset) | bits;
    this.writeRegister(offset, next);
    return next;
  }
  clearRegisterBits(offset, bits) {
    const next = this.readRegister(offset) & ~bits;
    this.writeRegister(offset, next);
    return next;
  }
  register(offset) {
    return new RegisterField(new Uint32Array(this.memory.buffer, offset, 1));
  }
  dumpState(sink) {
    sink.writeBytes(this.memory);
  }
  restoreState(source) {
    source.readBytes(this.memory);
  }
}
PeripheralBase.debug = false;

export class EmptyPeripheral extends PeripheralBase {}
