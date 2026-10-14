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
];
let bad = 0;
for (const [op, want] of CASES) {
  const got = classify(op).k === 'br' ? classify(op).b : classify(op).k;
  const wantK = ['bne', 'bge', 'bgeu', 'blt', 'bltu', 'beq', 'bnez', 'beqz', 'bnezn', 'bbci', 'beqtbl', 'bnetbl', 'bbsi', 'bbr', 'bltz', 'bgetbl', 'bban'].includes(want) ? want : want;
  if (got !== wantK) { console.log(`MISMATCH 0x${op.toString(16)}: got ${got}, want ${wantK}`); bad++; }
}
console.log(bad ? `CLASSIFY-CHECK FAILED (${bad})` : `CLASSIFY-CHECK passed (${CASES.length} cases)`);
process.exit(bad ? 1 : 0);
