import os
ws_path = 'build/webserver.bin'
if os.path.exists(ws_path):
    with open(ws_path, 'rb') as f:
        data = f.read()
    print(f'webserver.bin: {len(data)} bytes')
    print(f'first 32 hex: {data[:32].hex()}')
    for offset in [0, 0x1000, 0x8000, 0x9000, 0x10000]:
        if len(data) > offset:
            print(f'at 0x{offset:x}: {data[offset:offset+16].hex()}')
else:
    print('webserver.bin not found')
