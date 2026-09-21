import { PeripheralBase } from "./peripheral.js";

function getFieldValue(addr, value) {
  return (addr >> value.shift) & value.mask;
}
function clearFieldBits(addr, value) {
  return addr & ~(value.mask << value.shift);
}

let FIFO_SIZE = 128,
  REG_FIFO_DATA = 0,
  REG_INT_RAW = 4,
  REG_INT_ST = 8,
  REG_INT_ENA = 12,
  REG_INT_CLR = 16,
  REG_CLKDIV = 20,
  REG_CONF0 = 32,
  REG_CONF1 = 36,
  REG_STATUS = 28,
  BIT_AUTOBAUD_EN = 1,
  SHIFT_BIT_NUM = 2,
  MASK_BIT_NUM = 3,
  BIT_CONF_FLAG_4 = 4,
  BIT_TICK_REF_ON = 0x8000000,
  MASK_RX_FIFO_CNT = 255,
  SHIFT_RX_FIFO_CNT = 0,
  MASK_TX_FIFO_CNT = 255,
  SHIFT_TX_FIFO_CNT = 16,
  MASK_TX_STATE = 15,
  SHIFT_TX_STATE = 24,
  MASK_AT_CHAR = 255,
  SHIFT_AT_CHAR = 0,
  MASK_AT_NUM = 255,
  SHIFT_AT_NUM = 8,
  BIT_AT_CMD_DET = 262144,
  BIT_TX_DONE = 16384,
  BIT_RX_TIMEOUT = 256,
  BIT_TX_EMPTY_INT = 2,
  BIT_RX_FULL_INT = 1;

const TxStateMachine = {
  TX_IDLE: 0,
  TX_STRT: 1,
  TX_DAT0: 2,
  TX_DAT1: 3,
  TX_DAT2: 4,
  TX_DAT3: 5,
  TX_DAT4: 6,
  TX_DAT5: 7,
  TX_DAT6: 8,
  TX_DAT7: 9,
  TX_PRTY: 10,
  TX_STP1: 11,
  TX_STP2: 12,
  TX_DL0: 13,
  TX_DL1: 14,
};

class UartController extends PeripheralBase {
  constructor(chip, base, coreIndex, irq, clk, uartConfig) {
    (super(chip, base, coreIndex),
      (this.index = irq),
      (this.irq = clk),
      (this.config = uartConfig),
      (this.rxFIFO = []),
      (this.txFIFO = []),
      (this.rxFullThreshold = 0),
      (this.txEmptyThreshold = 0),
      (this.rxTimeoutThreshold = 10),
      (this.txState = TxStateMachine.TX_IDLE),
      (this.dataReceived = false),
      (this._baudRate = 0),
      (this.rxTimeout = this.cpu.clocks.cpu.createEvent(() => {
        (this.setRegisterBits(REG_INT_RAW, BIT_RX_TIMEOUT),
          this.checkInterrupt());
      })),
      (this.intCheckEvent = this.cpu.clocks.cpu.createEvent(() => {
        this.checkInterrupt();
      })),
      (this.atChar = 0),
      (this.atNum = 0),
      (this.atCounter = 0),
      (this.loopbackByte = -1),
      (this.autobaudEnabled = false),
      (this.autobaudRate = 115200),
      (this._glitchFilterEnabled = false),
      (this._glitchFilterThreshold = 0),
      (this.onTX = () => {}),
      (this.onConfigurationUpdated = () => {}),
      (this.onAutoBaudUpdated = () => {}),
      (this.onGlitchFilterUpdated = () => {}));
      uartConfig.clockSource?.addFrequencyListener(() => this.updateBaudRate());
  }
  get baudRate() {
    return this._baudRate;
  }

  get intStatus() {
    return this.readRegister(REG_INT_RAW) & this.readRegister(REG_INT_ENA);
  }
  checkInterrupt() {
    let { rxFullThreshold: addr, txEmptyThreshold: value } = this,
      regOffset =
        this.readRegister(REG_INT_RAW) & ~(BIT_RX_FULL_INT | BIT_TX_EMPTY_INT);
    (this.rxFIFO.length >= addr && (regOffset |= BIT_RX_FULL_INT),
      this.txFIFO.length <= value && (regOffset |= BIT_TX_EMPTY_INT),
      this.writeRegister(REG_INT_RAW, regOffset),
      this.cpu.interrupt(this.irq, 0 !== this.intStatus));
  }
  get clock() {
    let { clocks: addr } = this.cpu;
    if (this.config.clockSource) return this.config.clockSource;
    if (this.config.F.SCLK_SEL)
      switch (this.readField(this.config.F.SCLK_SEL)) {
        case 1:
        default:
          return addr.apb;
        case 2:
          return addr.rcFast;
        case 3:
          return addr.xtal;
      }
    return addr.ref
      ? this.readRegister(REG_CONF0) & BIT_TICK_REF_ON
        ? addr.apb
        : addr.ref
      : addr.apb;
  }
  get clockForAutoBaud() {
    let { clocks: addr } = this.cpu;
    return addr.ref ? addr.apb : this.clock;
  }
  get clockDiv() {
    if (this.config.clockSource) return 1;
    let { SCLK_DIV_NUM: addr } = this.config.F;
    return 1 + (addr ? this.readField(addr) : 0);
  }
  get glitchFilterThresholdNs() {
    if (!this._glitchFilterEnabled || 0 === this._glitchFilterThreshold)
      return 0;
    let addr = this.clockForAutoBaud.frequency / this.clockDiv;
    return Math.round((1e9 * this._glitchFilterThreshold) / addr);
  }
  get glitchFilterEnabled() {
    return this._glitchFilterEnabled;
  }
  updateGlitchFilter(addr, value) {
    let regOffset = false;
    (addr !== this._glitchFilterEnabled &&
      ((this._glitchFilterEnabled = addr), (regOffset = true)),
      undefined !== value &&
        value !== this._glitchFilterThreshold &&
        ((this._glitchFilterThreshold = value), (regOffset = true)),
      regOffset && this.onGlitchFilterUpdated(this));
  }
  updateBaudRate() {
    let addr = this.readRegister(REG_CLKDIV),
      value = 1048575 & addr,
      regOffset = (addr >> 20) & 15;
    ((this._baudRate = Math.floor(
      this.clock.frequency / this.clockDiv / (value + regOffset / 16),
    )),
      this.onConfigurationUpdated(this));
  }
  get txBusy() {
    return this.txState !== TxStateMachine.TX_IDLE;
  }
  get irdaTxSuppressed() {
    let { F: fieldCfg } = this.config;
    return (
      !!this.readField(fieldCfg.IRDA_EN) && !this.readField(fieldCfg.IRDA_TX_EN)
    );
  }
  txUpdated() {
    let { F: fieldCfg } = this.config;
    if (!this.txBusy && this.txFIFO.length) {
      if (this.irdaTxSuppressed) {
        ((this.txFIFO.length = 0),
          (this.loopbackByte = -1),
          this.setRegisterBits(REG_INT_RAW, BIT_TX_DONE),
          this.checkInterrupt());
        return;
      }
      let value = this.txFIFO.shift();
      (this.clearRegisterBits(REG_INT_RAW, BIT_TX_DONE),
        this.readField(fieldCfg.LOOPBACK)
          ? (this.loopbackByte = value)
          : (this.loopbackByte = -1),
        (this.txState = TxStateMachine.TX_STRT),
        this.onTX(value),
        this.txComplete());
    } else this.txFIFO.length || this.setRegisterBits(REG_INT_RAW, BIT_TX_DONE);
    this.checkInterrupt();
  }
  txComplete() {
    let { F: fieldCfg } = this.config;
    (this.readField(fieldCfg.LOOPBACK) &&
      this.loopbackByte >= 0 &&
      (this.feedByte(this.loopbackByte, true), (this.loopbackByte = -1)),
      (this.txState = TxStateMachine.TX_IDLE),
      this.txUpdated());
  }
  startDetected() {
    this.rxTimeout.unschedule();
  }
  get irdaRxSuppressed() {
    let { F: fieldCfg } = this.config;
    return (
      !!this.readField(fieldCfg.IRDA_EN) && !!this.readField(fieldCfg.IRDA_TX_EN)
    );
  }
  feedByte(addr, value = false) {
    if (
      !(
        !value &&
        (this.readField(this.config.F.LOOPBACK) || this.irdaRxSuppressed)
      )
    )
      return (
        addr === this.atChar
          ? (this.atCounter++,
            this.atCounter === this.atNum &&
              ((this.atCounter = 0),
              this.setRegisterBits(REG_INT_RAW, BIT_AT_CMD_DET),
              this.checkInterrupt()))
          : (this.atCounter = 0),
        this.rxFIFO.length !== FIFO_SIZE &&
          (this.rxFIFO.push(addr),
          (this.dataReceived = true),
          this.checkInterrupt(),
          this.rxTimeout.schedule(
            this.rxTimeoutThreshold * (1e9 / this.baudRate),
          ),
          true)
      );
  }
  get bitNum() {
    switch ((this.readRegister(REG_CONF0) >> SHIFT_BIT_NUM) & MASK_BIT_NUM) {
      case 0:
        return 5;
      case 1:
        return 6;
      case 2:
        return 7;
      default:
        return 8;
    }
  }
  updateAutoBaud(addr) {
    addr !== this.autobaudEnabled &&
      ((this.autobaudEnabled = addr), this.onAutoBaudUpdated(this));
  }
  readUint8(addr) {
    return 255 & this.readUint32(addr);
  }
  readUint16(addr) {
    return 65535 & this.readUint32(addr);
  }
  readUint32(addr) {
    let value = addr - this.baseAddr,
      { RmtChannelRegister: regOffset, F: fieldCfg } = this.config;
    switch (value) {
      case REG_FIFO_DATA: {
        let rxByte = this.rxFIFO.shift();
        if (null == rxByte) return 238;
        return (this.checkInterrupt(), rxByte);
      }
      case REG_INT_ST:
        return this.intStatus;
      case regOffset.LOWPULSE:
      case regOffset.HIGHPULSE:
        return Math.min(
          fieldCfg.LOWPULSE_MIN_CNT.mask,
          ((2 * this.clockForAutoBaud.frequency) / this.autobaudRate - 2) / 2,
        );
      case regOffset.NEGPULSE:
      case regOffset.POSPULSE:
        return Math.min(
          fieldCfg.NEGEDGE_MIN_CNT.mask,
          (2 * this.clockForAutoBaud.frequency) / this.autobaudRate - 1,
        );
      case regOffset.RXD_CNT:
        if (this.dataReceived) return 255;
        break;
      case REG_STATUS: {
        let addr =
          ((this.txFIFO.length & MASK_TX_FIFO_CNT) << SHIFT_TX_FIFO_CNT) |
          ((this.rxFIFO.length & MASK_RX_FIFO_CNT) << SHIFT_RX_FIFO_CNT);
        return (
          this.config.hasTXState &&
            (addr |= (this.txState & MASK_TX_STATE) << SHIFT_TX_STATE),
          addr
        );
      }
      case regOffset.MEM_RX_STATUS: {
        let addr = this.rxFIFO.length;
        return addr < FIFO_SIZE ? addr << 13 : 0;
      }
      case regOffset.ID:
        return 0x7fffffff & this.readRegister(value);
      case regOffset.REG_UPDATE:
        return 0;
    }
    return super.readUint32(addr);
  }
  writeUint8(addr, value) {
    if (addr - this.baseAddr === REG_FIFO_DATA) {
      this.txFIFO.length < FIFO_SIZE &&
        (this.txFIFO.push(value), this.txUpdated());
      return;
    }
    super.writeUint8(addr, value);
  }
  writeUint32(addr, value) {
    let regOffset = addr - this.baseAddr,
      { RmtChannelRegister: rmtCfg, F: fieldCfg } = this.config;
    if (regOffset === fieldCfg.RX_TOUT_THRHD.reg) {
      let addr =
        (value >> fieldCfg.RX_TOUT_THRHD.shift) &
        fieldCfg.RX_TOUT_THRHD.mask;
      if (this.config.toutMultiply) {
        let value = this.readRegister(REG_CONF0) & BIT_TICK_REF_ON;
        this.rxTimeoutThreshold = value ? 8 * addr : addr >> 3;
      } else this.rxTimeoutThreshold = addr;
    }
    (regOffset === fieldCfg.RXFIFO_RST.reg &&
      getFieldValue(value, fieldCfg.RXFIFO_RST) &&
      (this.rxFIFO.splice(0, this.rxFIFO.length),
      (this.dataReceived = false),
      this.checkInterrupt(),
      (value = clearFieldBits(value, fieldCfg.RXFIFO_RST))),
      regOffset === fieldCfg.TXFIFO_RST.reg &&
        getFieldValue(value, fieldCfg.TXFIFO_RST) &&
        (this.txFIFO.splice(0, this.txFIFO.length),
        this.txUpdated(),
        (value = clearFieldBits(value, fieldCfg.TXFIFO_RST))),
      fieldCfg.AUTOBAUD_EN &&
        regOffset === fieldCfg.AUTOBAUD_EN.reg &&
        this.updateAutoBaud(
          !!getFieldValue(value, fieldCfg.AUTOBAUD_EN),
        ));
    let sclkReg = fieldCfg.SCLK_SEL ? fieldCfg.SCLK_SEL.reg : -1;
    switch (regOffset) {
      case REG_FIFO_DATA:
        this.txFIFO.length < FIFO_SIZE &&
          (this.txFIFO.push(255 & value), this.txUpdated());
        return;
      case REG_INT_ENA:
        (super.writeRegister(REG_INT_ENA, value),
          this.intCheckEvent.schedule(100));
        return;
      case REG_INT_CLR:
        (this.clearRegisterBits(REG_INT_RAW, value),
          this.checkInterrupt(),
          this.cpu.interrupt(this.irq, 0 !== this.intStatus));
        return;
      case REG_CONF0:
        (super.writeUint32(addr, value), this.updateBaudRate());
        break;
      case REG_CONF1:
        ((this.rxFullThreshold = getFieldValue(
          value,
          fieldCfg.RXFIFO_FULL_THRHD,
        )),
          (this.txEmptyThreshold = getFieldValue(
            value,
            fieldCfg.TXFIFO_EMPTY_THRHD,
          )));
        break;
      case REG_CLKDIV:
      case sclkReg:
        (super.writeUint32(addr, value), this.updateBaudRate());
        return;
      case rmtCfg.RX_FILT:
        if (rmtCfg.RX_FILT >= 0) {
          let regOffset = fieldCfg.GLITCH_FILT
              ? getFieldValue(value, fieldCfg.GLITCH_FILT)
              : 0,
            rmtCfg = fieldCfg.GLITCH_FILT_EN
              ? !!getFieldValue(value, fieldCfg.GLITCH_FILT_EN)
              : regOffset > 0;
          (this.updateGlitchFilter(rmtCfg, regOffset),
            super.writeUint32(addr, value));
          return;
        }
        break;
      case rmtCfg.AT_CMD_CHAR:
        ((this.atChar = (value >> SHIFT_AT_CHAR) & MASK_AT_CHAR),
          (this.atNum = (value >> SHIFT_AT_NUM) & MASK_AT_NUM),
          (this.atCounter = 0),
          this.clearRegisterBits(REG_INT_RAW, BIT_AT_CMD_DET),
          this.checkInterrupt());
        return;
      case rmtCfg.AUTOBAUD:
        if (
          (this.updateAutoBaud(!!(value & BIT_AUTOBAUD_EN)),
          fieldCfg.GLITCH_FILT)
        ) {
          let addr = getFieldValue(value, fieldCfg.GLITCH_FILT);
          this.updateGlitchFilter(!!(value & BIT_AUTOBAUD_EN), addr);
        }
    }
    super.writeUint32(addr, value);
  }
  reset() {
    (super.reset(),
      this.writeRegister(
        REG_CONF0,
        BIT_TICK_REF_ON | (3 << SHIFT_BIT_NUM) | (1 << BIT_CONF_FLAG_4),
      ),
      this.writeRegister(REG_CLKDIV, 694));
    let { F: addr } = this.config,
      value = this.readRegister(REG_CONF1);
    ((this.rxFullThreshold = getFieldValue(value, addr.RXFIFO_FULL_THRHD)),
      (this.txEmptyThreshold = getFieldValue(
        value,
        addr.TXFIFO_EMPTY_THRHD,
      )),
      this.rxFIFO.splice(0, this.rxFIFO.length),
      this.txFIFO.splice(0, this.txFIFO.length),
      (this.atCounter = 0));
  }
}

export { UartController };

