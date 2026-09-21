import { generateBinary } from '../src/peripherals/common/partition-table.js';

const csv = `\
nvs,data,nvs,0x9000,0x5000
otadata,data,ota,0xe000,0x2000
ota_0,app,ota_0,0x10000,0x1E0000
ota_1,app,ota_1,0x200000,0x1E0000`;

// Import parseCSV to test the whole pipeline
import { parseCSV } from '../src/peripherals/common/partition-table.js';
const entries = parseCSV(csv);
console.log('Entries:');
entries.forEach((e, i) => console.log(`  ${i}: ${e.name} type=0x${e.type.toString(16)} subtype=0x${e.subtype.toString(16)} offset=0x${e.offset.toString(16)} size=0x${e.size.toString(16)}`));

const bin = generateBinary(entries);
const totalBytes = bin.length;
console.log(`\nTotal binary size: ${totalBytes} bytes (${totalBytes/32} entries)`);
console.log('\nHex dump:');
for (let i = 0; i < totalBytes; i += 16) {
  const hex = Array.from(bin.subarray(i, i + 16)).map(b => b.toString(16).padStart(2, '0')).join(' ');
  console.log(`${i.toString(16).padStart(4, '0')}: ${hex}`);
}

console.log('\nDecoded:');
for (let i = 0; i < totalBytes; i += 32) {
  const magic = bin.readUInt16LE(i);
  const type = bin[i+2];
  const subtype = bin[i+3];
  const addr = bin.readUInt32LE(i+4);
  const size = bin.readUInt32LE(i+8);
  const label = bin.subarray(i+12, i+28).toString('utf8').replace(/\0.*$/s, '');
  console.log(`  entry ${i/32}: magic=0x${magic.toString(16).padStart(4,'0')} type=0x${type.toString(16).padStart(2,'0')} subtype=0x${subtype.toString(16).padStart(2,'0')} addr=0x${addr.toString(16).padStart(8,'0')} size=0x${size.toString(16).padStart(8,'0')} label="${label}"`);
}
