// Extracted from index.js module 11882

export class ESPTrace {
  constructor() {
    this.traces = [];
    this.uartBuf = ["", ""];
    this.traceReturns = false;
    this.traceMemWrites = false;
    this.dumpThreshold = 1e5;
  }
  traceLog(direction, data, meta) {
    this.traces.push(
      `SignalDirection,${direction},${JSON.stringify(data)},${JSON.stringify(meta)}`,
    );
  }
  traceUartTx(core, byte) {
    if (byte === 10) {
      const line = this.uartBuf[core];
      this.traces.push(`ClockEvent,${core},${JSON.stringify(line)}`);
      this.uartBuf[core] = "";
    } else if (this.uartBuf[core].length < 255) {
      this.uartBuf[core] += String.fromCharCode(byte);
    }
  }
  traceEntry(pc, insn, meta) {
    if (!this.traceReturns) return;
    this.traces.push(`e,${pc},${insn},${meta}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  traceReturn(pc, insn, meta, cycles) {
    if (!this.traceReturns) return;
    this.traces.push(`r,${pc},${insn},${meta},${cycles}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  int(_pc, _insn) {}
  iret(_pc, _insn) {}
  traceMemWrite(addr, size, value, oldValue, newValue) {
    if (!this.traceMemWrites) return;
    this.traces.push(`w,${addr},${size},${value},${oldValue},${newValue}`);
    if (this.traces.length > this.dumpThreshold) this.dumpTrace();
  }
  dumpTrace() {
    this.traces = [];
  }
}
