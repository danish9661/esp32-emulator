// GDB Session class — extracted from index.js

import { byteToHex, hexToBytes, bytesToHex, uint32ToHex } from "./helpers.js";
import { ReadonlyMemory } from "./memory.js";

let breakResponse = makeGdbPacket("S02"),
  formatStopReply = (threadId) =>
    makeGdbPacket(`T05thread:${uint32ToHex(threadId)};`);
function computeChecksum(packet) {
  return byteToHex(
    255 &
      packet
        .split("")
        .map((ch) => ch.charCodeAt(0))
        .reduce((sum, code) => sum + code, 0),
  );
}
function makeGdbPacket(payload) {
  return `+$${payload}#${computeChecksum(payload)}`;
}
let verboseLogging = false,
  textDecoder = new TextDecoder();
class GdbSession {
  constructor(sim, target, transport, threadCtrl) {
    ((this.simulator = sim),
      (this.esp32 = target),
      (this.transport = transport),
      (this.threadController = threadCtrl),
      (this.currentCore = 0),
      (this.currentThread = 0),
      (this.buf = ""),
      (this.targetXml = target.gdbTargetXml),
      transport.onData(this.onData.bind(this)),
      this.updateCurrentThread(0));
  }
  updateCurrentThread(coreIdx) {
    ((this.currentCore = coreIdx), (this.currentThread = coreIdx + 1));
    let foundThread = this.threadController
      ?.threads()
      ?.find((t) => t.core === coreIdx);
    foundThread && (this.currentThread = foundThread.IntStatusAlias);
  }
  onBreak(coreIdx) {
    (this.updateCurrentThread(coreIdx),
      this.transport.write(formatStopReply(this.currentThread)));
  }
  handleCommand(cmd) {
    let {
        simulator: sim,
        esp32: target,
        targetXml: targetXml,
        threadController: threadCtrl,
      } = this,
      core = target.cores[this.currentCore];
    if (cmd.startsWith("qSupported"))
      return makeGdbPacket(
        "PacketSize=1000;qXfer:threads:read+" +
          (targetXml.length > 0 ? ";qXfer:features:read+" : ""),
      );
    if (cmd.startsWith("qAttached")) return makeGdbPacket("1");
    if (cmd.startsWith("qfThreadInfo")) {
      let threadList = threadCtrl?.threads() || [];
      return threadList.length > 0
        ? makeGdbPacket(
            "m" + threadList.map((t) => t.IntStatusAlias).join(","),
          )
        : target.cores.length > 1
          ? makeGdbPacket("m 1,2")
          : makeGdbPacket("m 1");
    }
    if (cmd.startsWith("qsThreadInfo"))
      return makeGdbPacket("l");
    if (cmd.startsWith("qXfer:threads:read::")) {
      let threadList = threadCtrl?.threads() || [];
      return threadList.length > 0
        ? makeGdbPacket(`l<?xml version="1.0"?>
      <threads>
        ${threadList.map((t) => `<thread IntStatusAlias="${t.IntStatusAlias.toString(16)}">${t.name}</thread>`).join("\n")}
      </threads>
    `)
        : target.cores.length > 1
          ? makeGdbPacket(`l<?xml version="1.0"?>
      <threads>
      <thread IntStatusAlias="1">Name: esp32.PRO</thread>
      <thread IntStatusAlias="2">Name: esp32.APP</thread>
      </threads>`)
          : makeGdbPacket(`l<?xml version="1.0"?>
<threads>
</threads>`);
    } else if (cmd.startsWith("qXfer:features:read:target.xml:")) {
      let [, , , , params] = cmd.split(":"),
        [offsetStr, lenStr] = params.split(","),
        offset = parseInt(offsetStr, 16),
        byteLen = parseInt(lenStr, 16);
      return makeGdbPacket(
        (targetXml.length > offset + byteLen ? "m" : "l") +
          targetXml.substring(offset, offset + byteLen),
      );
    } else if (cmd.startsWith("qRcmd,")) {
      let decoded = Array.from(hexToBytes(cmd.split(",")[1]))
        .map((byte) => String.fromCharCode(byte))
        .join("");
      return (console.log(decoded),
      "system_reset" === decoded ||
        "reset" === decoded ||
        "reset halt" === decoded)
        ? (target.reset(), makeGdbPacket("OK"))
        : makeGdbPacket("E00");
    } else if ("?" === cmd) return makeGdbPacket("S05");
    else if ("qC" === cmd)
      return makeGdbPacket(`QC ${uint32ToHex(this.currentThread)}`);
    else if ("qOffsets" === cmd) return makeGdbPacket("Text=0;Data=0;Bss=0");
    else if (cmd.startsWith("Hc")) {
      let threadId = parseInt(cmd.substring(2), 16);
      return 0 === threadId || (threadId >= 1 && threadId <= target.cores.length)
        ? makeGdbPacket("OK")
        : makeGdbPacket(
            threadCtrl?.threads().find(
              (t) => t.IntStatusAlias === threadId,
            )
              ? "OK"
              : "E00",
          );
    } else if (cmd.startsWith("Hg")) {
      let threadId = parseInt(cmd.substring(2), 16);
      if (
        (threadCtrl?.threads() || []).find(
          (t) => t.IntStatusAlias === threadId,
        )
      )
        this.currentThread = threadId;
      else {
        if (!(threadId >= 1) || !(threadId <= target.cores.length))
          return makeGdbPacket("E00");
        ((this.currentCore = threadId - 1), (this.currentThread = threadId));
      }
      return makeGdbPacket("OK");
    } else if ("s" === cmd) {
      do core.runInstruction();
      while (core.isWindowInstruction());
      return makeGdbPacket("S05");
    } else if ("c" === cmd) return (sim.execute(), null);
    else if ("g" === cmd) {
      let regs = new Uint32Array(core.gdbRegisterCount),
        useThreadRegs =
          this.threadController && this.currentThread > this.esp32.cores.length;
      for (let i = 0; i < regs.length; i++)
        regs[i] =
          this.threadController && useThreadRegs
            ? this.threadController.readRegister(this.currentThread, i)
            : core.gdbReadRegister(i);
      return makeGdbPacket(bytesToHex(new Uint8Array(regs.buffer)));
    } else if (cmd.startsWith("G")) {
      let regs = new Uint32Array(hexToBytes(cmd.substr(1)).buffer);
      if (!core || !(regs.length >= core.gdbRegisterCount))
        return makeGdbPacket("E00");
      for (let i = 0; i < regs.length; i++)
        core.gdbWriteRegister(i, regs[i]);
      return makeGdbPacket("OK");
    } else if (cmd.startsWith("m")) {
      let [addrStr, lenStr] = cmd.substring(1).split(","),
        addr = parseInt(addrStr, 16),
        len = parseInt(lenStr, 16);
      if (!len || len >= 65536)
        return makeGdbPacket("E05");
      if ((3 & addr) === 0 && (3 & len) === 0) {
        let data = new Uint32Array(len >> 2);
        for (let i = 0; i < data.length; i++)
          data[i] = core.readUint32(addr + 4 * i);
        return makeGdbPacket(bytesToHex(new Uint8Array(data.buffer)));
      }
      {
        let data = new Uint8Array(len);
        for (let i = 0; i < len; i++)
          data[i] = core.readUint8(addr + i);
        return makeGdbPacket(bytesToHex(data));
      }
    } else if (cmd.startsWith("M")) {
      let [addrStr, lenStr, dataHex] = cmd.substring(1).split(/[,:]/),
        addr = parseInt(addrStr, 16),
        len = parseInt(lenStr, 16),
        bytes = hexToBytes(dataHex).slice(0, len);
      {
        let savedOverride = ReadonlyMemory.override;
        if (
          ((ReadonlyMemory.override = true),
          (3 & addr) === 0 && (3 & len) === 0)
        ) {
          let words = new Uint32Array(bytes.buffer);
          for (let i = 0; i < words.length; i++)
            core.writeUint32(addr + 4 * i, words[i]);
        } else
          for (let i = 0; i < len; i++)
            core.writeUint8(addr + i, bytes[i]);
        return ((ReadonlyMemory.override = savedOverride), makeGdbPacket("OK"));
      }
    } else if (cmd.startsWith("T")) {
      let threadId = parseInt(cmd.substring(1), 16),
        foundThread = (threadCtrl?.threads() || []).find(
          (t) => t.IntStatusAlias === threadId,
        ),
        appCore = target.cores[1];
      return 1 === threadId ||
        null != foundThread ||
        (2 === threadId && appCore?.enabled)
        ? makeGdbPacket("OK")
        : makeGdbPacket("XtsState 00");
    } else if (cmd.startsWith("Z0,")) {
      let addr = parseInt(cmd.split(",")[1], 16);
      for (let core of target.cores)
        (core.breakpoints || (core.breakpoints = {}),
          (core.breakpoints[addr] = (coreIdx) => (
            target.onBreak?.(coreIdx),
            true
          )));
      return makeGdbPacket("OK");
    } else if (cmd.startsWith("z0,")) {
      let addr = parseInt(cmd.split(",")[1], 16);
      for (let core of target.cores)
        core.breakpoints && delete core.breakpoints[addr];
      return makeGdbPacket("OK");
    } else if (cmd.startsWith("Z2,")) {
      let addr = parseInt(cmd.split(",")[1], 16);
      return (target.writeWatchPoints.add(addr), makeGdbPacket("OK"));
    } else {
      if (!cmd.startsWith("z2,")) return makeGdbPacket("");
      let addr = parseInt(cmd.split(",")[1], 16);
      return (target.writeWatchPoints.delete(addr), makeGdbPacket("OK"));
    }
  }
  onData(data) {
    for (
      3 === data[0] &&
        (verboseLogging && console.log("BREAK"),
        this.simulator.stop(),
        this.transport.write(breakResponse),
        this.updateCurrentThread(0),
        (data = data.subarray(1))),
        this.buf += textDecoder.decode(data);
      ;
    ) {
      let start = this.buf.indexOf("$"),
        end = this.buf.indexOf("#");
      if (
        start < 0 ||
        end < 0 ||
        end < start ||
        end + 2 > this.buf.length
      )
        return;
      let payload = this.buf.substring(start + 1, end),
        checksum = this.buf.substring(end + 1, end + 3);
      if (
        ((this.buf = this.buf.substring(end + 2)),
        computeChecksum(payload) !== checksum)
      )
        (console.warn("Warning: GDB checksum error in message:", payload),
          this.transport.write("-"));
      else {
        (this.transport.write("+"), verboseLogging && console.log(">", payload));
        let response = this.handleCommand(payload);
        response &&
          (verboseLogging && console.log("<", response),
          this.transport.write(response));
      }
    }
  }
}

export { GdbSession };

