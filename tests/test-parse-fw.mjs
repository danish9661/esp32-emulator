import { readFileSync } from 'fs';
const bin = readFileSync('firmware.bin');
console.log('=== Flash layout ===');
console.log('0x0000:', [...bin.subarray(0, 16)].map(b=>b.toString(16).padStart(2,'0')).join(' '));
console.log('0x1000:', [...bin.subarray(0x1000, 0x1010)].map(b=>b.toString(16).padStart(2,'0')).join(' '));
console.log('');
console.log('=== Partition table ===');
for (let i = 0x8000; i < 0x80f0; i += 32) {
  const magic = bin.readUInt16LE(i);
  const type = bin[i+2];
  const subtype = bin[i+3];
  const addr = bin.readUInt32LE(i+4);
  const size = bin.readUInt32LE(i+8);
  const label = bin.subarray(i+12, i+28).toString('utf8').replace(/\0.*$/s, '');
  console.log(i.toString(16) + ': magic=0x' + magic.toString(16).padStart(4,'0') + ' type=0x' + type.toString(16).padStart(2,'0') + ' subtype=0x' + subtype.toString(16).padStart(2,'0') + ' addr=0x' + addr.toString(16).padStart(8,'0') + ' size=0x' + size.toString(16).padStart(8,'0') + ' label="' + label + '"');
}
