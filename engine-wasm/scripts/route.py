# Route a narrow opcode through c_handler7's REAL decision tree (parsed from
# handlers.rs) to its handler name. Eliminates hand-walk mask misreads.
# (Regenerated 2026-10-11 after /tmp wipe; lives in-repo now.)
import re
import sys

SRC = '/home/danish1075/Documents/esp32 emu/engine-wasm/src/xtensa/handlers.rs'


def load_tree():
    lines = open(SRC).read().splitlines()
    start = next(i for i, l in enumerate(lines) if 'pub fn c_handler7' in l)
    depth = 0
    end = start
    started = False
    for i in range(start, len(lines)):
        for ch in lines[i]:
            if ch == '{':
                depth += 1
                started = True
            elif ch == '}':
                depth -= 1
        if started and depth == 0:
            end = i
            break
    return lines[start:end + 1]


def translate(lines):
    out = ['def route(tmp):']
    indent = 1
    stack = []
    for ln in lines[1:]:
        s = ln.strip()
        if not s or s.startswith('//'):
            continue
        m = re.match(r'match tmp_val & (0x[0-9a-fA-F]+|\d+) \{$', s)
        if m:
            out.append('  ' * indent + f'_m = tmp & {m.group(1)}')
            continue
        m = re.match(r'if \(tmp_val & ([^)]+)\) == ([^ \{]+) \{\s*([A-Za-z_0-9]+)\(core, tmp_val\);\s*return;\s*\}$', s)
        if m:
            out.append('  ' * indent + f'if (tmp & ({m.group(1).strip()})) == ({m.group(2).strip()}): return {m.group(3).strip()!r}')
            continue
        m = re.match(r'if \(tmp_val & ([^)]+)\) == ([^ ]+) \{$', s)
        if m:
            out.append('  ' * indent + f'if (tmp & ({m.group(1).strip()})) == ({m.group(2).strip()}):')
            indent += 1
            stack.append('if')
            continue
        m = re.match(r'((?:0x[0-9a-fA-F]+|\d+)(?:\s*\|\s*(?:0x[0-9a-fA-F]+|\d+))*)\s*=>\s*\{\s*([A-Za-z_0-9]+)\(core, tmp_val\);\s*return;\s*\}$', s)
        if m:
            labels = [int(x.strip(), 0) for x in m.group(1).split('|')]
            out.append('  ' * indent + f'if _m in {set(labels)}: return {m.group(2)!r}')
            continue
        m = re.match(r'((?:0x[0-9a-fA-F]+|\d+)(?:\s*\|\s*(?:0x[0-9a-fA-F]+|\d+))*)\s*=>\s*\{\s*return;\s*\}$', s)
        if m:
            labels = [int(x.strip(), 0) for x in m.group(1).split('|')]
            out.append('  ' * indent + f'if _m in {set(labels)}: return "NOP"')
            continue
        m = re.match(r'((?:0x[0-9a-fA-F]+|\d+)(?:\s*\|\s*(?:0x[0-9a-fA-F]+|\d+))*|_)\s*=>\s*\{$', s)
        if m:
            lab = m.group(1).strip()
            if lab == '_':
                out.append('  ' * indent + 'if True:  # default')
            else:
                labels = [int(x.strip(), 0) for x in lab.split('|')]
                out.append('  ' * indent + f'if _m in {set(labels)}:')
            indent += 1
            stack.append('arm')
            continue
        if s == '}':
            if stack:
                stack.pop()
                indent -= 1
            continue
        if s == '_ => {}':
            continue
    out.append('  ' * 1 + 'return "ILLEGAL"')
    return '\n'.join(out)


def main():
    tree = load_tree()
    py = translate(tree)
    ns = {}
    exec(py, ns)
    route = ns['route']
    ops = [int(x, 0) for x in sys.argv[1:]]
    for op in ops:
        print('0x%x -> %s' % (op, route(op)))


if __name__ == '__main__':
    main()
