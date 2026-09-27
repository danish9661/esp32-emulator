# Bumble virtual BLE controller over TCP (for the ESP32 emu).
#
# Listens on 127.0.0.1:14821 for HCI clients. Speaks Bumble's TCP transport
# framing (type-prefixed HCI packets) with a real `bumble.controller.Controller`
# behind it, so RESET / LE ADV / GATT-bearing commands get architecturally
# correct Command Complete / Command Status / event answers.
#
# A virtual LE peer (own controller+host+Device on the same LocalLink) keeps
# a GATT server with one readable characteristic (UUID 0xFF01 = b'HELLO')
# and scans continuously; discovered ESP32 advertisements are logged as
# ADVRPT lines. A JSON control channel on the same TCP stream supports
# {"op":"connect","address":...} and {"op":"read","uuid":"FF01"}.
#
# Usage:  python3 tests/test-bumble-hci.py [--port 14821]
# Test:   python3 tests/test-bumble-hci.py --selftest   (no ESP32 needed)
import argparse
import asyncio
import json
import logging
import sys

sys.path.insert(0, '/home/danish1075/.local/lib/python3.14/site-packages')

from bumble import core as bumble_core  # noqa: E402
from bumble import hci  # noqa: E402
from bumble.device import Device, Peer  # noqa: E402
from bumble.gatt import Characteristic, CharacteristicValue, Service  # noqa: E402
from bumble.host import Host  # noqa: E402
from bumble.controller import Controller  # noqa: E402
from bumble.link import LocalLink  # noqa: E402
from bumble.transport.common import (  # noqa: E402
    PacketParser,
)

LOG = logging.getLogger('test-bumble-hci')


def hci_frame(packet_type: int, payload: bytes) -> bytes:
    return bytes([packet_type]) + payload


class EngineSide(asyncio.Protocol):
    """One TCP connection = one HCI peer (the emulator's VHCI proxy)."""

    def __init__(self, controller: Controller, get_peer, get_peer_ctrl):
        self.controller = controller
        self._get_peer = get_peer
        self._get_peer_ctrl = get_peer_ctrl
        self.parser = PacketParser(sink=self)
        self.transport = None
        self.rx_count = 0
        self.tx_count = 0
        self.conn = None

    def peer(self):
        return self._set_peer()

    def connection_made(self, transport):
        self.transport = transport
        LOG.info('engine connected from %s', transport.get_extra_info('peername'))

    def on_packet(self, packet: bytes):
        # PacketParser emits the full HCI packet (type byte first).
        # Controller dispatches commands/ACL by type below.
        self.rx_count += 1
        ptype = packet[0]
        LOG.info('H2C #%d type=0x%02x len=%d %s', self.rx_count, ptype,
                 len(packet), packet[:16].hex())
        if ptype == hci.HCI_COMMAND_PACKET:
            cmd = hci.HCI_Command.from_bytes(packet)
            self.controller.on_hci_command_packet(cmd)
        elif ptype == hci.HCI_ACL_DATA_PACKET:
            self.controller.on_hci_acl_data_packet(
                hci.HCI_AclDataPacket.from_bytes(packet))
        else:
            LOG.warning('unhandled H2C packet type 0x%02x', ptype)

    def handle_control(self, packet: bytes):
        """Control channel: JSON requests arrive as {..}\\n lines (see
        data_received); replies go back as JSON lines terminated by \\n."""
        try:
            req = json.loads(packet.decode())
        except Exception as e:
            self.send_json({'ok': False, 'error': f'bad json: {e}'})
            return
        op = req.get('op')
        LOG.info('CTL op=%s', op)
        loop = asyncio.get_running_loop()
        if op == 'connect':
            addr = req.get('address', 'AA:BB:CC:DD:EE:FF')
            loop.create_task(self.do_connect(addr), name='do-connect')
        elif op == 'read':
            loop.create_task(self.do_read(req), name='do-read')
        else:
            self.send_json({'ok': False, 'error': f'unknown op {op}'})

    def send_json(self, obj):
        if self.transport:
            self.transport.write((json.dumps(obj) + '\n').encode())

    async def do_connect(self, addr: str):
        peer = self.peer()
        LOG.info('do_connect peer=%s', 'present' if peer is not None else 'NONE')
        if peer is None:
            self.send_json({'ok': False, 'error': 'no peer yet'})
            return
        try:
            # START FRESH: a previous attempt leaves BOTH the Device flag
            # (le_connecting) and the controller slot (pending_le_connection)
            # wedged, because LocalLink never delivers without an advertiser
            # on the other side. Bumble's cancel path sends
            # LE_Create_Connection_Cancel, which the controller answers with
            # CC only — the pending slot clears on Connection Complete,
            # which never comes. Clear both sides, then stop scanning (a
            # second stop is harmless) before the fresh attempt.
            peer.le_connecting = False
            ctrl = self.peer_ctrl()
            if ctrl is not None:
                ctrl.pending_le_connection = None
            await peer.stop_scanning()
            # Pass a parsed Address so connect() skips the by-name scan
            # (our ADV payload has no local name; find_peer_by_name would
            # hang until timeout).
            target = hci.Address(addr)
            conn = await peer.connect(target,
                                      transport=bumble_core.PhysicalTransport.LE,
                                      timeout=20.0)
            self.conn = conn
            self.send_json({'ok': True, 'event': 'connected',
                            'handle': conn.handle,
                            'address': str(addr)})
        except Exception as e:
            LOG.info('do_connect exc=%s', str(e)[:120])
            self.send_json({'ok': False, 'event': 'connect-failed',
                            'error': str(e)[:200]})

    async def do_read(self, req):
        conn = getattr(self, 'conn', None)
        if conn is None:
            self.send_json({'ok': False, 'error': 'not connected'})
            return
        try:
            uuid = req.get('uuid', 'FF01')
            svcs = await conn.gatt_client.discover_services()
            found = None
            for s in svcs:
                for c in await conn.gatt_client.discover_characteristics(
                        service=s):
                    if str(c.uuid).upper().endswith(uuid.upper()):
                        found = c
            if found is None:
                self.send_json({'ok': False, 'error': 'char not found',
                                'services': [str(s.uuid) for s in svcs]})
                return
            val = await found.read_value()
            self.send_json({'ok': True, 'event': 'read',
                            'uuid': str(found.uuid), 'value': bytes(val).hex()})
        except Exception as e:
            self.send_json({'ok': False, 'event': 'read-failed',
                            'error': str(e)[:200]})

    def peer(self):
        return self._get_peer()

    def peer_ctrl(self):
        return self._get_peer_ctrl()

    def send_to_engine(self, packet: bytes):
        self.tx_count += 1
        LOG.info('C2H #%d len=%d %s', self.tx_count, len(packet),
                 packet[:16].hex())
        if self.transport:
            self.transport.write(packet)

    def data_received(self, data: bytes):
        # Control channel: JSON lines (one {..}\n per line) are handled
        # here, before HCI framing — PacketParser would reject b'{' as a
        # packet type. self._ctlbuf accumulates a partial line. Anything
        # that is not a JSON line is HCI bytes and goes to the parser
        # VERBATIM (never split on \n: 0x0A is a legal HCI payload byte).
        buf = getattr(self, '_ctlbuf', b'') + data
        if buf.startswith(b'{'):
            # JSON control line: wait for the terminating newline. (HCI
            # packets never start with b'{' = 0x7B: no HCI packet type is
            # 0x7B — types are 0x01..0x05.)
            nl = buf.find(b'\n')
            if nl < 0:
                self._ctlbuf = buf
                return
            line, buf = buf[:nl], buf[nl + 1:]
            line = line.strip()
            if line.startswith(b'{'):
                self.handle_control(line)
            self._ctlbuf = buf
            if not buf:
                return
        try:
            self.parser.feed_data(buf)
            self._ctlbuf = b''
        except Exception:
            self._ctlbuf = buf

    def connection_lost(self, exc):
        LOG.info('engine disconnected')


async def selftest(port: int):
    """Drive the controller directly (no TCP): RESET + LE ADV enable."""
    c2h = []

    class Sink:
        def on_packet(self, packet: bytes):
            c2h.append(packet)

    link = LocalLink()
    ctrl = Controller('emu0', host_source=None, host_sink=Sink(), link=link)

    def cmd(op, params=b''):
        return (bytes([hci.HCI_COMMAND_PACKET]) + op.to_bytes(2, 'little')
                + bytes([len(params)]) + bytes(params))

    seq = [
        (hci.HCI_RESET_COMMAND, b''),
        (hci.HCI_LE_SET_ADVERTISING_PARAMETERS_COMMAND, bytes(15)),
        (hci.HCI_LE_SET_ADVERTISING_DATA_COMMAND,
         bytes([4, 0x41, 0x44, 0x56, 0x31] + [0] * 26)),
        (hci.HCI_LE_SET_SCAN_RESPONSE_DATA_COMMAND, bytes(31)),
        (hci.HCI_LE_SET_ADVERTISING_ENABLE_COMMAND, bytes([1])),
    ]
    for op, params in seq:
        raw = cmd(op, params)
        ctrl.on_hci_command_packet(hci.HCI_Command.from_bytes(raw))
        await asyncio.sleep(0.05)
    await asyncio.sleep(0.2)
    expected = [
        '040e0401030c00',
        '040e0401062000',
        '040e0401082000',
        '040e0401092000',
        '040e04010a2000',
    ]
    actual = [p.hex() for p in c2h]
    print('C2H packets:', len(actual))
    for a in actual:
        print('  ', a)
    assert actual == expected, f'MISMATCH:\n{actual}\nvs\n{expected}'
    assert ctrl.le_legacy_advertiser.enabled, 'legacy advertiser not enabled'
    print('SELFTEST PASS: RESET CC + ADV CCs + advertiser enabled')


async def peer_task(link: LocalLink, state: dict):
    """Virtual LE peer: own controller+host wired in-process, GATT server
    with one readable characteristic, always scanning. Discovered ESP32
    advertisements are logged as ADVRPT lines (the path a real central
    would take); the TCP client can later trigger connect/read."""
    loop = asyncio.get_running_loop()
    holder = {}

    class CtrlSink:
        def on_packet(self, packet: bytes):
            loop.call_soon(holder['host'].on_packet, packet)

    ctrl = Controller('peer0', host_source=None, host_sink=CtrlSink(),
                      link=link)
    host = Host(controller_source=None, controller_sink=None)
    holder['host'] = host

    class HostToCtrl:
        def on_packet(self, packet: bytes):
            ptype = packet[0]
            if ptype == hci.HCI_COMMAND_PACKET:
                ctrl.on_hci_command_packet(hci.HCI_Command.from_bytes(packet))
            elif ptype == hci.HCI_ACL_DATA_PACKET:
                ctrl.on_hci_acl_data_packet(
                    hci.HCI_AclDataPacket.from_bytes(packet))

    host.hci_sink = HostToCtrl()
    peer = Device(name='ble-peer', address=hci.Address('F0:F1:F2:F3:F4:F5'),
                  host=host)
    ch = Characteristic(
        bumble_core.UUID(0xFF01),
        Characteristic.Properties.READ,
        Characteristic.READABLE,
        CharacteristicValue(read=lambda _conn: b'HELLO'),
    )
    peer.add_service(Service(bumble_core.UUID(0x00FF), [ch]))
    await peer.power_on()
    LOG.info('peer powered on')
    state['peer_dev'] = peer
    state['peer_ctrl'] = ctrl

    def on_adv(adv):
        data = bytes(adv.data) if adv.data else b''
        LOG.info('ADVRPT addr=%s data=%s', adv.address, data.hex())

    peer.on('advertisement', on_adv)
    await peer.start_scanning()
    LOG.info('peer scanning')
    return peer


async def serve(port: int):
    link = LocalLink()
    loop = asyncio.get_running_loop()
    state = {}
    # Peer boots at server start (not on first engine connection) so the
    # LocalLink has both controllers before any HCI flows.
    state['ctrl'] = Controller('emu0', host_source=None,
                               host_sink=None, link=link,
                               public_address=hci.Address(
                                   'AA:BB:CC:DD:EE:FF'))
    loop.create_task(peer_task(link, state))

    def make_proto():
        proto = EngineSide(state['ctrl'], lambda: state.get('peer_dev'),
                           lambda: state.get('peer_ctrl'))
        # Rebind controller C2H output to this connection.
        orig_send = state['ctrl'].send_hci_packet

        def send_and_forward(packet):
            proto.send_to_engine(bytes(packet))
        state['ctrl'].send_hci_packet = send_and_forward
        return proto

    server = await loop.create_server(make_proto, '127.0.0.1', port)
    LOG.info('listening on 127.0.0.1:%d', port)
    async with server:
        await server.serve_forever()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--port', type=int, default=14821)
    ap.add_argument('--selftest', action='store_true')
    args = ap.parse_args()
    logging.basicConfig(level=logging.INFO, format='%(name)s %(message)s')
    if args.selftest:
        asyncio.run(selftest(args.port))
    else:
        asyncio.run(serve(args.port))


if __name__ == '__main__':
    main()
