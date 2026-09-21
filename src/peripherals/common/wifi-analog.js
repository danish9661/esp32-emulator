// wifi-analog - extracted from engine-peripherals.js

export function applyPeripheralResetValues(chip, resetSpec) {
  for (const peripheral of chip.peripherals) peripheral.zeroMemory();
  const core = chip.cores[0];
  for (let i = 0; i < resetSpec.length; i += 2) {
    const addr = resetSpec[i];
    const resetList = resetSpec[i + 1];
    for (const [off, val, count, stride] of resetList) {
      for (let k = 0; k < count; k++) {
        core.writeUint32(addr + off + k * stride, val);
      }
    }
  }
}

export function applySingleResetValues(peripheral, resetSpec) {
  peripheral.zeroMemory();
  for (let i = 0; i < resetSpec.length; i += 2) {
    const addr = resetSpec[i];
    const resetList = resetSpec[i + 1];
    if (addr === peripheral.baseAddr) {
      for (const [off, val, count, stride] of resetList) {
        for (let k = 0; k < count; k++) {
          peripheral.writeUint32(addr + off + k * stride, val);
        }
      }
    }
  }
}
