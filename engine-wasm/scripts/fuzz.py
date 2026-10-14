# Fuzz: JS classify mirror vs router over the full 24-bit narrow op space.
# Contract: classify==unknown always safe; any claimed shape must route to
# its transcribed handler. In-repo so /tmp wipes can't kill it.
import sys
import time
from route import load_tree, translate

code = translate(load_tree())
ns2 = {}
exec(code, ns2)
route = ns2['route']


def sext8(v):
    return (v | 0xffffff00) - 0x100000000 if v & 128 else v


def classify(op):
    if (op & 0xffff0f) == 0x60c0:
        return ('unknown',)
    if (op & 0xff) == 0x1d and ((op >> 8) & 0xff) == 0xf0 and ((op >> 16) & 0xf0) == 0x30:
        return ('unknown',)
    lo = op & 15
    f12 = (op >> 12) & 15
    f8 = (op >> 8) & 15
    f4 = (op >> 4) & 15
    if lo == 8:
        return ('l32in',)
    if lo == 9:
        return ('s32in',)
    if lo == 10:
        return ('addN',)
    if lo == 11:
        return ('addiN',)
    if lo == 12 and not (op & 128):
        return ('moviN',)
    if lo == 12 and (op & 128) and not (op & 64):
        return ('br-beqz33',)
    if lo == 12 and (op & 128) and (op & 64):
        return ('bnezn',)
    if lo == 13 and (op & 61440) == 61440 and (op & 240) == 16:
        return ('callEnd',)
    if lo == 13 and (op & 61440) == 0:
        return ('mov',)
    if lo == 2 and (op & 61440) == 0:
        return ('l8ui',)
    if lo == 2 and (op & 61440) == 40960:
        return ('movi',)
    if lo == 2 and (op & 61440) == 53248:
        return ('addmi',)
    if lo == 2 and (op & 61440) == 16384:
        return ('s8i',)
    if lo == 2 and (op & 61440) == 49152:
        return ('addi8',)
    if lo == 2 and (op & 61440) == 36864:
        return ('l16i',)
    if lo == 2 and (op & 61440) == 8192:
        return ('l32i8',)
    if lo == 2 and (op & 61440) == 24576:
        return ('s32i8',)
    if lo == 1:
        return ('l32r',)
    if lo == 7 and (op & 57344) == 32768 and (op & 4096) == 4096:
        return ('br-bne',)
    if lo == 7 and (op & 57344) == 8192 and (op & 4096) == 4096:
        return ('br-bltu',)
    if lo == 7 and (op & 57344) == 0 and (op & 4096) == 4096:
        return ('br-beq',)
    if lo == 7 and (op & 57344) == 40960:
        return ('br-bgeu',) if (op & 4096) else ('br-bge',)
    if lo == 7 and (op & 57344) == 8192 and not (op & 4096):
        return ('br-blt',)
    if lo == 7 and (op & 57344) == 24576:
        return ('br-bbci',)
    if lo == 7 and (op & 57344) == 57344:
        return ('br-bbsi',)
    if lo == 5:
        return ('callEnd',)
    if lo == 6 and (op & 48) == 16 and (op & 192) == 64:
        return ('br-bnez',)
    if lo == 6 and (op & 48) == 16 and (op & 192) == 0:
        return ('br-beqz32',)
    if lo == 6 and (op & 48) == 32 and (op & 192) == 64:
        return ('br-bnetbl',)
    if lo == 6 and (op & 48) == 32 and (op & 192) == 0:
        return ('br-beqtbl',)
    if lo == 6 and (op & 48) == 48 and (op & 192) == 0:
        return ('callEnd',)
    if lo == 6 and (op & 48) == 0:
        return ('j',)
    if lo == 6 and (op & 48) == 48 and (op & 192) == 64:
        if (op & 61440) in (32768, 36864, 40960):
            return ('loopEnd',)
    if lo == 0:
        e = op & 0xE0000
        f = op & 0xE10000
        b20 = op & 0x100000
        if e == 0x20000 and f == 0x800000 and b20 == 0:
            return ('mull',)
        if e == 0 and f == 0x810000 and b20 == 0:
            return ('src',)
        if e == 0x40000:
            return ('extui',)
        if e == 0 and (op & 0xE10000) == 0 and b20 != 0:
            return ('and',)
        if e == 0 and f == 0x200000 and b20 == 0:
            return ('or',)
        if e == 0 and f == 0x410000:
            return ('srl',)
        if e == 0 and f == 0x200000 and b20 != 0:
            return ('xor',)
        if e == 0x80000 and (op & 0xF10000) == 0:
            return ('fload',)
        if e == 0 and f == 0xC00000 and b20 == 0:
            return ('sub',)
        if e == 0x20000 and (op & 0xF10000) == 65536:
            return ('rsrcc',)
        if e == 0 and f == 0x400000 and (op & 1110016) == 16384:
            return ('ssai',)
        if e == 0 and f == 65536:
            return ('slli',)
        if e == 0 and f == 2162688:
            return ('srai',)
        if e == 0 and f == 0xA00000 and b20 == 0:
            return ('addx2',)
        if e == 0 and f == 0 and b20 == 0 and (op & 61440) == 8192 and (op & 4080) in (0, 16, 32, 48, 192, 208, 240):
            return ('nop',)
    return ('unknown',)


EXPECT = {
    'l32in': 'r_handler47', 'l32i8': 'r_handler46', 's32i8': '_handler20',
    'br-bbci': 'n_handler27', 'srl': '_handler39', 'xor': 'c_handler4',
    'br-beqtbl': 'n_handler31', 'fload': 'r_handler59',
    'br-bnetbl': 'n_handler47', 'br-bbsi': 'n_handler29', 'l16i': 'r_handler42', 's8i': '_handler16', 's32in': '_handler21', 'addN': 'n_handler9',
    'addiN': 'n_handler12', 'moviN': 'a_handler13', 'l32r': 'r_handler48',
    'br-bne': 'n_handler46', 'br-bltu': 'n_handler42', 'br-beq': 'n_handler30',
    'br-bge': 'n_handler35', 'br-bgeu': 'n_handler37', 'br-blt': 'n_handler40',
    'br-bnez': 'n_handler48', 'br-beqz32': 'n_handler32', 'br-beqz33': 'n_handler33',
    'bnezn': 'n_handler49',
    'movi': 'a_handler12', 'addmi': 'n_handler13', 'addi8': 'n_handler11',
    'mov': 'a_handler4', 'l8ui': 'r_handler41',
    'mull': 'a_handler37', 'src': '_handler37', 'extui': 'r_handler26',
    'and': 'n_handler19', 'or': 'a_handler51', 'sub': '_handler49',
    'rsrcc': '_handler14', 'ssai': '_handler42',
    'slli': '_handler31', 'srai': '_handler36', 'addx2': 'n_handler15',
    'nop': 'NOP', 'j': 'r_handler39',
}
BOUND = {
    'callEnd': {'r_handler0', 'r_handler1', 'r_handler2', 'r_handler3',
                'r_handler6', 'r_handler24', 'a_handler62'},
    'loopEnd': {'r_handler54', 'r_handler55', 'r_handler56'},
}

misroutes = []
covered = {}
count = 0
t0 = time.time()
for hi in range(256):
    base = hi << 16
    for lo16 in range(65536):
        op = base | lo16
        c = classify(op)[0]
        if c == 'unknown':
            continue
        h = route(op)
        if c in BOUND:
            if h not in BOUND[c]:
                misroutes.append((op, c, h))
            else:
                covered[c] = covered.get(c, 0) + 1
            continue
        exp = EXPECT[c]
        if h != exp:
            misroutes.append((op, c, h))
        else:
            covered[c] = covered.get(c, 0) + 1
        count += 1
        if len(misroutes) >= 20:
            break
    if len(misroutes) >= 20:
        break
    if hi % 64 == 0:
        print('hi=%d/256 misroutes=%d (%.0fs)' % (hi, len(misroutes), time.time() - t0), flush=True)
print('done: %d classified, %d misroutes (%.0fs)' % (count, len(misroutes), time.time() - t0))
for m in misroutes[:20]:
    print('MISROUTE op=0x%x shape=%s router=%s' % m)
print('coverage:', {k: v for k, v in sorted(covered.items())})
