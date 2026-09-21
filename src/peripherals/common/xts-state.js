// xts-state - XTS flash-encryption state holder (live: used by esp32.js this.xts)
import { XtsState } from "./enums.js";

export class XtsEncryptionState {
  constructor() {
    this.state = XtsState.IDLE;
    this.lineSize = 0;
    this.destination = 0;
    this.physicalAddress = 0;
  }

  reset() {
    this.state = XtsState.IDLE;
    this.lineSize = 0;
    this.destination = 0;
    this.physicalAddress = 0;
  }
}
