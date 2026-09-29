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

# ATT uplink fix (2026-09-28): this Bumble's gatt_server.Server.on_gatt_pdu
# calls async ATT handlers (on_att_read_by_group_type_request etc.) WITHOUT
# awaiting them -> "coroutine was never awaited", NO response is ever sent,
# peer GATT discovery times out. Peer-side Device works only because...
# actually it does NOT work either in-proc (acl_probe2-5 all time out on
# discover_services) — the ONLY in-proc success (acl_probe6) ran power_on
# first, which... no. Root-cause precisely: on_gatt_pdu is sync `def`,
# handlers are `async def`; handler(bearer, pdu) returns a coroutine that
# is dropped. Schedule async handlers as tasks. Verified acl_probe29:
# discover + READ VALUE 48454c4c4f after patch. Applied at import so BOTH
# peer and emu GATT servers answer.
try:
    import inspect as _inspect
    from bumble import gatt_server as _gsmod

    _orig_on_gatt_pdu = _gsmod.Server.on_gatt_pdu

    def _patched_on_gatt_pdu(self, bearer, att_pdu):
        from bumble import att as _attmod
        _handler = getattr(self, f'on_{att_pdu.name.lower()}', None)
        if _handler is not None and _inspect.iscoroutinefunction(_handler):
            asyncio.get_running_loop().create_task(_handler(bearer, att_pdu))
            return
        return _orig_on_gatt_pdu(self, bearer, att_pdu)

    _gsmod.Server.on_gatt_pdu = _patched_on_gatt_pdu
except Exception as _e:
    print('gatt_server patch FAILED:', str(_e)[:120])

LOG = logging.getLogger('test-bumble-hci')


def ts():
    import datetime
    return datetime.datetime.now().strftime('%H:%M:%S.%f')[:-3]


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
        LOG.info('%s engine connected from %s', ts(),
                 transport.get_extra_info('peername'))

    def on_packet(self, packet: bytes):
        # PacketParser emits the full HCI packet (type byte first).
        # Controller dispatches commands/ACL by type below.
        # H2C at INFO with wall-clock (the TX-ack dispute of 2026-09-28
        # proved per-packet INFO is load-bearing for correlation: the
        # engine's TX timestamps vs the bridge's RX timestamps decide
        # whether bytes crossed the socket. ADVRPT stays capped at 8.)
        self.rx_count += 1
        ptype = packet[0]
        LOG.info('%s H2C #%d type=0x%02x len=%d %s', ts(), self.rx_count,
                 ptype, len(packet), packet[:16].hex())
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
                # Drop any stale CENTRAL connection to this peer (a prior
                # connect attempt that completed at the controller level
                # but whose JSON reply was lost leaves le_connections[addr]
                # populated; create_le_connection then hits "already
                # exists?" and returns WITHOUT completing -> the awaiting
                # peer.connect() hangs to timeout and no reply is ever
                # sent. Observed 2026-09-28: 5.4M-line spam storm).
                try:
                    stale = ctrl.le_connections.pop(
                        hci.Address(addr), None)
                    if stale is not None:
                        LOG.info('do_connect dropped stale conn handle=%s',
                                 getattr(stale, 'handle', '?'))
                except Exception as e:
                    LOG.info('do_connect stale-drop exc=%s', str(e)[:80])
            LOG.info('do_connect stop_scanning...')
            await peer.stop_scanning()
            LOG.info('do_connect stopped, connecting to %s...', addr)
            # Pass a parsed Address so connect() skips the by-name scan
            # (our ADV payload has no local name; find_peer_by_name would
            # hang until timeout).
            target = hci.Address(addr)
            conn = await peer.connect(target,
                                      transport=bumble_core.PhysicalTransport.LE,
                                      timeout=20.0)
            LOG.info('do_connect connected handle=%s', conn.handle)
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
                        [], s):
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
        LOG.info('%s C2H #%d len=%d %s', ts(), self.tx_count, len(packet),
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
        LOG.info('%s engine disconnected', ts())
        # Drop this connection's eng controller from the shared link so
        # its stale le_connections/pending state can never answer the
        # NEXT run's air traffic (observed: run2 ATT routed to run1's
        # dead eng_ctrl -> GATT timeout; plus "reuses the same handle"
        # + stale-conn drop spam). peer_link is reachable via any live
        # controller's .link; guard everything (disconnect races setup).
        try:
            _ctrl = getattr(self, 'controller', None)
            _link = getattr(_ctrl, 'link', None)
            if _link is not None and _ctrl is not None:
                try:
                    _link.remove_controller(_ctrl)
                    LOG.info('dropped dead eng controller from link')
                except Exception as _e2:
                    LOG.info('drop dead ctrl FAILED: %s', str(_e2)[:80])
        except Exception as _e:
            LOG.info('drop dead ctrl outer FAILED: %s', str(_e)[:80])


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
    # Emu-side GATT server: the ESP32 emulator never answers ATT (its LL
    # posts HCI commands only; no GATT server runs in emulated firmware),
    # so peer-to-emu ATT requests would time out even though the air path
    # Emu-side GATT server: NOT here (see make_proto). It must be wired to
    # the per-connection eng controller AFTER make_proto creates it: wiring
    # it here finds no 'emu0' yet (peer_task runs before any engine
    # connection), and powering a Device on a foreign controller RESETs it
    # (proven 2026-09-28: acl_probe8 hung with zero output when the emu
    # Device powered on before eng ctrl existed on the link).
    state['emu_gatt_wanted'] = True

    def on_adv(adv):
        data = bytes(adv.data) if adv.data else b''
        # ADVRPT flood-guard (2026-09-28): the peer logs EVERY air ADV at
        # INFO; one ADV_ENABLE burst = ~900k lines and the bridge process
        # spins at 5% CPU writing them. Cap at 8 lines per bridge process
        # lifetime — enough to prove over-the-air delivery, then silent
        # (the radio stays live; only the log is gated).
        if getattr(on_adv, 'n', 0) < 8:
            on_adv.n = getattr(on_adv, 'n', 0) + 1
            LOG.info('ADVRPT addr=%s data=%s', adv.address, data.hex())

    peer.on('advertisement', on_adv)
    await peer.start_scanning()
    LOG.info('peer scanning')
    return peer


async def emu_dev_setup(state, eng_ctrl):
    """Emu-side GATT server setup (async task): keeps make_proto sync.
    Manual wiring WITHOUT Device.power_on: power_on sends HCI_RESET down
    the controller, which (a) races the emu's own H2C RESET dialog and
    (b) routes the RESET CC to the emu Host instead of the TCP engine.
    Order matters (all proven 2026-09-28):
      1. ehost FIRST + emu_holder['host'] set, else the send_hci_packet
         override fires with no host -> 'emu-host forward exc=host'.
      2. EHostToCtrl uplink bridge (ehost.hci_sink) so ATT responses
         reach the air path (else dropped, acl_probe22).
      3. Device + service, ACL queues from eng ctrl sizes, ehost.ready.
      4. eng_ctrl.host sink + send_hci_packet override (copy to emu
         Host SYNC, then TCP-forward). Sync: Connection Complete must
         exist BEFORE the peer's ATT arrives ~ms later.
      5. random_address = public addr (else peer drops our replies:
         'no connection for 00:00...', acl_probe25).
      6. create_le hook (backup Host/Dev Conn creation).
      7. emu_emit_sync direct l2cap dispatch (same-tick ATT answer).
    Proven in-proc (acl_probe27): discover + read 48454c4c4f."""
    try:
        from bumble.host import DataPacketQueue
        emu_holder = {}
        loop = asyncio.get_running_loop()

        # 1. Host FIRST so the forward override never sees an empty holder.
        ehost = Host(controller_source=None, controller_sink=None)
        emu_holder['host'] = ehost

        # 2. Uplink bridge: ATT responses flow ehost -> EHostToCtrl ->
        # Link Connection (by controller handle map) -> LocalLink air.
        class EHostToCtrl:
            def on_packet(self, packet: bytes):
                ptype = packet[0]
                if ptype == hci.HCI_COMMAND_PACKET:
                    eng_ctrl.on_hci_command_packet(
                        hci.HCI_Command.from_bytes(packet))
                elif ptype == hci.HCI_ACL_DATA_PACKET:
                    acl = hci.HCI_AclDataPacket.from_bytes(packet)
                    conn = eng_ctrl.find_connection_by_handle(
                        acl.connection_handle)
                    if conn is None:
                        LOG.info('emu uplink: no ctrl conn for handle %d',
                                 acl.connection_handle)
                        return
                    conn.on_hci_acl_data_packet(acl)

        ehost.hci_sink = EHostToCtrl()

        # 3. Device + service + queues + ready.
        emu_dev = Device(name='emu-dev',
                         address=hci.Address('AA:BB:CC:DD:EE:FF'),
                         host=ehost)
        emu_dev.add_service(Service(
            bumble_core.UUID(0x00FF), [Characteristic(
                bumble_core.UUID(0xFF01),
                Characteristic.Properties.READ,
                Characteristic.READABLE,
                CharacteristicValue(read=lambda _conn: b'HELLO'))]))
        ehost.acl_packet_queue = DataPacketQueue(
            max_packet_size=eng_ctrl.acl_data_packet_length,
            max_in_flight=eng_ctrl.total_num_acl_data_packets,
            send=ehost.send_hci_packet,
        )
        ehost.le_acl_packet_queue = DataPacketQueue(
            max_packet_size=eng_ctrl.le_acl_data_packet_length,
            max_in_flight=eng_ctrl.total_num_le_acl_data_packets,
            send=ehost.send_hci_packet,
        )
        ehost.ready = True
        state['emu_dev'] = emu_dev

        # 4. Downlink: sync COPY to emu Host, then TCP-forward original.
        # EmuCtrlSink is DIRECT (no call_soon): stock send_hci_packet
        # already defers via call_soon, so a second deferral pushes
        # Connection Complete past the peer's ATT arrival (observed:
        # emu host conns=[] at ATT time). Direct call here = single
        # deferral, matching probe27/28 which got conns=[1].
        class EmuCtrlSink:
            def on_packet(self, packet: bytes):
                emu_holder['host'].on_packet(packet)

        # NOTE: Controller has NO host_sink attribute — the sink field is
        # hci_sink, set via the .host property. Use .host.
        eng_ctrl.host = EmuCtrlSink()
        _emu_tcp_forward = eng_ctrl.send_hci_packet

        def emu_send_and_forward(packet):
            try:
                emu_holder['host'].on_packet(bytes(packet))
            except Exception as e:
                LOG.info('emu-host forward exc=%s', str(e)[:80])
            _emu_tcp_forward(packet)

        eng_ctrl.send_hci_packet = emu_send_and_forward

        # 5. Link source-address fix (power_on would set this).
        try:
            eng_ctrl.random_address = hci.Address('AA:BB:CC:DD:EE:FF')
            LOG.info('eng random_address set to public addr')
        except Exception as e:
            LOG.info('eng random_address set FAILED: %s', str(e)[:80])

        # 6. Backup Host/Dev Connection creation (primary path is the
        # forwarded Connection Complete event -> Host handler ->
        # host.connections + Device.on_le_connection -> dev.connections).
        _orig_create_le = eng_ctrl.create_le_connection

        def emu_create_le_and_host_conn(peer_address):
            _orig_create_le(peer_address)
            try:
                from bumble.device import Connection as DevConn
                _emu_dev = state.get('emu_dev')
                for _addr, _lc in eng_ctrl.le_connections.items():
                    _h = _lc.handle
                    if _h not in emu_holder['host'].connections:
                        from bumble.host import Connection as HostConn
                        from bumble.core import PhysicalTransport as PhysT
                        emu_holder['host'].connections[_h] = HostConn(
                            emu_holder['host'], _h, _lc.peer_address,
                            PhysT.LE)
                        LOG.info('emu-host connection created handle=%d', _h)
                    if _emu_dev is not None and _h not in _emu_dev.connections:
                        _emu_dev.connections[_h] = DevConn(
                            device=_emu_dev,
                            handle=_h,
                            transport=bumble_core.PhysicalTransport.LE,
                            self_address=_lc.self_address,
                            self_resolvable_address=None,
                            peer_address=_lc.peer_address,
                            peer_resolvable_address=None,
                            role=hci.Role.PERIPHERAL,
                            parameters=DevConn.Parameters(0.0, 0, 0.0))
                        LOG.info('emu-dev connection created handle=%d', _h)
            except Exception as e:
                LOG.info('emu-host conn create exc=%s', str(e)[:120])

        eng_ctrl.create_le_connection = emu_create_le_and_host_conn

        # 7. (REMOVED 2026-09-28: direct l2cap_pdu dispatch double-
        # delivered every ATT response — the stock emit path already
        # reaches the Device handler, so the extra direct call made the
        # GATT server answer twice -> peer "InvalidStateError: invalid
        # state" x18/run. Stock emit path only.)
        # 8. Publish the CURRENT binding so the NEXT make_proto call can
        # rebind it to the fresh eng controller (see binding handoff in
        # make_proto). Without this the 2nd run's ATT uplink hits the 1st
        # run's dead eng_ctrl (observed: run1 HELLO PASS, run2 GATT
        # timeout — EHostToCtrl closed over the stale controller).
        state['emu_bind'] = {
            'emu_holder': emu_holder,
            'ehost': ehost,
            'emu_dev': emu_dev,
            'tcp_forward': _emu_tcp_forward,
        }
        LOG.info('emu-side GATT server up (0x00FF/0xFF01=HELLO, no RESET)')
    except Exception as e:
        LOG.info('emu-side GATT server FAILED: %s', str(e)[:160])


async def serve(port: int):
    loop = asyncio.get_running_loop()
    state = {}
    # Peer boots at server start on its own permanent link (peer_link);
    # each engine connection gets a FRESH controller on a FRESH link that
    # is then bridged to the peer link. Rationale (observed 2026-09-28):
    # a single shared Controller accumulates H2C sequence state across
    # engine reconnects (every simulator run opens a new TCP connection),
    # so run N's RESET replays against run N-1's post-RESET state and the
    # replies go stale. A fresh controller per connection behaves like a
    # freshly-powered radio every run.
    peer_link = LocalLink()
    loop.create_task(peer_task(peer_link, state))

    def make_proto():
        # Fresh engine-side link + controller per connection (see above).
        # The peer's advertisements must reach the engine controller and
        # vice versa: bridge by forwarding each link's air packets to the
        # other. LocalLink exposes `add_controller`/`remove_controller`;
        # the simplest correct bridge that works across bumble versions is
        # to share ONE link object per connection pair — but the peer link
        # is permanent, so instead create the engine controller directly
        # ON the peer link (same radio medium, fresh controller state).
        # Fresh Controller, same LocalLink: new HCI sequence state, same
        # air as the peer.
        eng_ctrl = Controller('emu0', host_source=None,
                              host_sink=None, link=peer_link,
                              public_address=hci.Address(
                                  'AA:BB:CC:DD:EE:FF'))
        proto = EngineSide(eng_ctrl, lambda: state.get('peer_dev'),
                           lambda: state.get('peer_ctrl'))

        def send_and_forward(packet):
            proto.send_to_engine(bytes(packet))
        eng_ctrl.send_hci_packet = send_and_forward
        # Binding handoff (2nd+ TCP run): the FIRST run's emu_dev_setup
        # published state['emu_bind'] = shared ehost/emu_dev/holder. The
        # fresh eng_ctrl object needs the SAME wiring re-pointed at it
        # (host sink, send_hci_packet override, random_address, create_le
        # hook, EHostToCtrl uplink closure) — else run N+1's ATT uplink
        # hits run 1's dead controller (observed: run1 HELLO PASS, run2
        # GATT timeout). Reuse the SAME ehost/emu_dev (no RESET, no
        # re-power); only swap the controller reference. Runs synchronously
        # here (make_proto is sync, all ops are plain attribute swaps).
        _bind = state.get('emu_bind')
        if _bind is not None and _bind.get('ehost') is not None:
            try:
                _ehost = _bind['ehost']
                _holder = _bind['emu_holder']
                _emu_dev = _bind.get('emu_dev')

                class _EHostToCtrl2:
                    def on_packet(self, packet: bytes):
                        ptype = packet[0]
                        if ptype == hci.HCI_COMMAND_PACKET:
                            eng_ctrl.on_hci_command_packet(
                                hci.HCI_Command.from_bytes(packet))
                        elif ptype == hci.HCI_ACL_DATA_PACKET:
                            acl = hci.HCI_AclDataPacket.from_bytes(packet)
                            conn = eng_ctrl.find_connection_by_handle(
                                acl.connection_handle)
                            if conn is None:
                                return
                            conn.on_hci_acl_data_packet(acl)

                _ehost.hci_sink = _EHostToCtrl2()

                class _EmuCtrlSink2:
                    def on_packet(self, packet: bytes):
                        _holder['host'].on_packet(packet)

                eng_ctrl.host = _EmuCtrlSink2()
                _tcp_fwd = eng_ctrl.send_hci_packet

                def _emu_send_and_forward2(packet, _f=_tcp_fwd):
                    try:
                        _holder['host'].on_packet(bytes(packet))
                    except Exception:
                        pass
                    _f(packet)

                eng_ctrl.send_hci_packet = _emu_send_and_forward2
                try:
                    eng_ctrl.random_address = hci.Address(
                        'AA:BB:CC:DD:EE:FF')
                except Exception:
                    pass
                _orig2 = eng_ctrl.create_le_connection

                def _emu_create_le2(peer_address, _o=_orig2):
                    _o(peer_address)
                    try:
                        from bumble.device import Connection as DevConn
                        from bumble.host import Connection as HostConn
                        from bumble.core import PhysicalTransport as PhysT
                        for _addr, _lc in eng_ctrl.le_connections.items():
                            _h = _lc.handle
                            if _h not in _holder['host'].connections:
                                _holder['host'].connections[_h] = HostConn(
                                    _holder['host'], _h, _lc.peer_address,
                                    PhysT.LE)
                            if (_emu_dev is not None
                                    and _h not in _emu_dev.connections):
                                _emu_dev.connections[_h] = DevConn(
                                    device=_emu_dev,
                                    handle=_h,
                                    transport=PhysT.LE,
                                    self_address=_lc.self_address,
                                    self_resolvable_address=None,
                                    peer_address=_lc.peer_address,
                                    peer_resolvable_address=None,
                                    role=hci.Role.PERIPHERAL,
                                    parameters=DevConn.Parameters(
                                        0.0, 0, 0.0))
                    except Exception:
                        pass

                eng_ctrl.create_le_connection = _emu_create_le2
                LOG.info('emu binding handed off to new eng controller')
            except Exception as _e:
                LOG.info('emu binding handoff FAILED: %s', str(_e)[:120])
        # Emu-side GATT server (per-connection, AFTER eng_ctrl exists):
        # the ESP32 emulator never answers ATT (its LL posts HCI commands
        # only; no GATT server runs in emulated firmware), so peer-to-emu
        # ATT requests would time out even though the air path works.
        # Serve the SAME 0x00FF/0xFF01=HELLO database from a virtual
        # emu-side Device wired to THIS connection's eng controller: the
        # peer's ATT requests then complete over the LocalLink air path,
        # proving RF delivery end-to-end. Must live here (not peer_task):
        # peer_task runs before any engine connection exists (no 'emu0'
        # on the link yet), and powering a Device against a controller
        # that later gets replaced RESETs its state (observed 2026-09-28:
        # acl_probe8 hung with zero output in that order; acl_probe6 with
        # eng-ctrl-first completed discover + read 48454c4c4f).
        # Only the FIRST connection sets it up (one server per bridge).
        if state.get('emu_gatt_wanted') and not state.get('emu_dev'):
            state['emu_gatt_wanted'] = False
            # emu_dev_setup wires Host + ATT + GATT DB with NO RESET
            # (see fn docs); safe at TCP-accept time.
            loop.create_task(emu_dev_setup(state, eng_ctrl))
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
