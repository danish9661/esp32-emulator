// Regression guard: lib classify must return these exact shapes.
// Catches brace-nesting/insertion damage that routing fuzz (mirror) can't see.
import { classify } from './tmp-jit-lib.mjs';
const CASES = [
  [0x218, 'l32in'], [0x1349, 's32in'], [0x75a820, 'extui'], [0x1022a0, 'and'],
  [0x3ea20, 'rsrcc'], [0xFF9DE7, 'bne'], [0x404600, 'ssai'], [0xfa5381, 'l32r'],
  [0x18280, 'slli'], [0x219890, 'srai'], [0xa02320, 'addx2'], [0x982, 'l8ui'],
  [0x20c0, 'nop'], [0xDCC, 'bnezn'], [0x2212, 'l32i8'], [0xFFA687, 'bge'],
  [0xFFB687, 'bgeu'], [0xFF2687, 'blt'], [0xFFFFC6, 'j'], [0x65D, 'mov'],
  [0x10C652, 'addi8'], [0x551b, 'addiN'], [0x778a, 'addN'], [0x827740, 'mull'],
  [0xC6A092, 'movi'], [0x983C, 'moviN'], [0x661B, 'addiN'], [0xFFB687, 'bgeu'],
  [0x16682, 's32i8'], [0x66397, 'bbci'], [0x418290, 'srl'], [0x308290, 'xor'],
  [0x5a080, 'extui'], [0x458180, 'extui'], [0x288, 'l32in'], [0x3228, 'l32in'],
  [0xf71826, 'beqtbl'], [0x82800, 'fload'], [0x9682, 'l16i'], [0x20466, 'bnetbl'],
  [0xf1e807, 'bbsi'], [0x4382, 's8i'],
  [0x932930, 'movnez'], [0x838660, 'moveqz'], [0x803390, 'addN'], // n_handler7 reuses addN emission
  [0xec897, 'bbr'], [0xfe7296, 'bltz'],
  [0x1ea2, 'l16u'],
  [0x5592, 's16i'], [0x602020, 'neg'], [0x3087f6, 'bgetbl'], [0x65387, 'bban'],
  [0x40f550, 'nsa'],
  [0xa23360, 'muluhf'],
  [0x401400, 'wsar'], [0xa1aa00, 'ssl'],
  [0xe0a990, 'subx4'], [0xf0a990, 'subx8'],
  [0x518e6, 'bgetbls'], [0x3fe502, 's32c1i'],
  [0xb4387, 'bbrn'], [0x638200, 'minu'], [0x90, 'callEnd'],
  [0x49ad6, 'bgez12'], [0xabd00, 'fadd'], [0x2ad00, 'band'], [0xc33020, 'movbr'],
  [0xb0aaf0, 'addx8'],
  [0x732a80, 'maxu'], [0x5399a0, 'maxs'], [0xc22a80, 'divu'], [0x5bb800, 'fcmp'], [0x238903, 'lsiu'], [0xf2b100, 'rems'], [0x220000, 'bor'], [0x239a00, 'sexpn'], [0x9a2500, 'ftrunc'],
  // RSR / RSRV: exact masks (hi 0x03 / 0x61|0x71, lo nibble must be 0)
  [0x30c80, 'rsr'], [0x3ebb0, 'rsr'], [0x610c20, 'rsrv'], [0x613600, 'rsrv'],
  [0x71fff0, 'rsrv'], [0x3038f, 'unknown'], [0x620000, 'unknown'],
];
let bad = 0;
for (const [op, want] of CASES) {
  const got = classify(op).k === 'br' ? classify(op).b : classify(op).k;
  const wantK = ['bne', 'bge', 'bgeu', 'blt', 'bltu', 'beq', 'bnez', 'beqz', 'bnezn', 'bbci', 'beqtbl', 'bnetbl', 'bbsi', 'bbr', 'bltz', 'bgetbl', 'bban', 'bgetbls', 'bgez12'].includes(want) ? want : want;
  if (got !== wantK) { console.log(`MISMATCH 0x${op.toString(16)}: got ${got}, want ${wantK}`); bad++; }
}
console.log(bad ? `CLASSIFY-CHECK FAILED (${bad})` : `CLASSIFY-CHECK passed (${CASES.length} cases)`);
process.exit(bad ? 1 : 0);
