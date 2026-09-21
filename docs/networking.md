# WiFi / wireless networking

The emulator runs the real ESP32 LWIP and WiFi-driver code on real 802.11
frames. The Rust WiFi port bridges Ethernet frames over a WebSocket to the
gateway (`openhw-studio-gateway/openhw-gw`, default `ws://127.0.0.1:5085`),
which serves DHCP/DNS (8.8.8.8), NATs TCP/UDP to the host network (gVisor
userspace stack — no root needed, ICMP included), and port-forwards
`127.0.0.1:8080` → board `:80` (TCP) plus `127.0.0.1:5683` → board `:5683`
(UDP, for server roles on the board). Start it before any WiFi test
(`openhw-studio-gateway/start-gateway.sh`) and **restart it fresh** before
WiFi runs — its NAT/state ages over many sessions (stale mappings were
observed to blackhole inbound UDP after ~10 runs).

Boards join gateway rooms via `wifi: { ..., room: '<name>' }` (defaults to
a random isolated room). The room is an L2 hub: frames are broadcast to
member boards, which is what carries ESP-NOW between nodes.

## What works (all verified live)

| Area | Protocol | Evidence |
|---|---|---|
| Station | scan, connect, DHCP, IPv4, ARP | `test-worker-wifi`, tcpdump-readable pcap |
| L3 | ICMP echo → gateway AND internet | `PING OK` both, replies in pcap |
| L4 | UDP (NTP ×2 servers) | `NTP=OK`, `NTP2=OK` |
| L5 | DNS A records via 8.8.8.8 | `DNS=<ip>` |
| L7 client | HTTP (`example.com`, 200 + body) | `HTTP=200 body=559 OK` |
| L7 client | MQTT CONNECT → CONNACK (`test.mosquitto.org:1883`) | `MQTT=20020000 OK` |
| L7 client | CoAP CON GET → ACK 2.05, ×3 legs | `COAP_EARLY/LATE/INET=PASS` |
| L7 server | HTTP server + live curl via TCP forward | `HTTP 200 (944B)` — `test-worker-webserver` |
| L7 server | CoAP server (bind 5683, ACK 2.05 both ways) | `test-worker-coap-server`: `COAP_RX=3 HOST_ACKS=3` |
| Soft-AP | `softAP()` init, `softAPIP()`, MAC, station count | `test-worker-softap` (no gateway needed) |
| ESP-NOW | init/peer/send/TXCB + two-node delivery + RXCB | `test-worker-espnow`: A↔B `RXCB=68656c6c6f` |
| IPv6 | SLAAC ULA, ICMPv6 echo, UDP echo | `test-worker-ipv6`: `SLAAC=OK PING6=OK UDP6=PASS` |

CoAP against the public internet also works (Californium answers our
request), but no public CoAP server is reliably reachable from here, so the
client test runs its own responder (Node `dgram`) and targets the host LAN
IP through the gateway NAT.

## ESP-NOW medium design

ESP-NOW action frames (mgmt type 0 / subtype 13 / category `0x7F` / OUI
`18:FE:34`) are snooped in the native TXDMA path (`snoop_espnow_action`,
offsets relative to `buf[0]` — this config has `tx_header: false`) and
handed to the host via the `js_espnow_tx_frame` FFI. The worker
magic-prefixes them (`E5 50 4E 57`) into the shared gateway room; the
gateway broadcasts room frames to peers but never feeds marked frames to
gVisor; the peer strips the magic and injects via the normal
`native_wifi_mac_rx_frame` DMA path, and the real driver fires RXCB.
Unencrypted delivery verified both directions; encrypted peers are
untested (driver-side CCMP should pass through opaquely).

## IPv6 (board <-> gateway)

IPv6 works between board and gateway: link-local + SLAAC global ULA
(`fd00::/64` via gateway RAs), ICMPv6 echo both ways, UDP echo. The
gateway crafts RA/NA/echo/UDP replies by hand (`openhw-studio-gateway/handleIPv6.go`;
the gVisor stack is v4-NAT only). Internet IPv6 egress is NOT implemented.

Implementation notes (earned the hard way):
- RAs are **unicast** to the soliciter: multicast-destined RAs never
  surface past the driver/LWIP multicast filter in this stack.
- Finite PIO lifetimes (86400/14400) + periodic unicast re-advertisement
  every 30s: sim time runs ~1000x wall-clock in idle phases, so lifetimes
  must be refreshed wall-clock-side or SLAAC addresses expire mid-test.
- IPv6 header field offsets (RFC 8200 §3): ver/TC/flow, len [18:20],
  next [20], hop [21], src [22:38], dst [38:54] — verify against the RFC,
  never against your own earlier code.
- `sin6_scope_id` is the LWIP netif IFINDEX = `num+1` (station num=1, so
  scope **2**, not 1 — scope 1 routes into the loopback and frames vanish).
- The Arduino `globalIPv6()` wrapper lags LWIP state (reports `::` with a
  VALID preferred GUA present) — the test asserts LWIP ground truth.
- `LWIP_IPV6_NUM_ADDRESSES` is small (not 8!): never walk past it or the
  firmware hard-faults mid-print.

## What does NOT work / is untested

- **IPv6 internet egress** (beyond the gateway): no v6 NAT/route in the
  gateway; no RA-triggered DNS (no RDNSS option sent).
- **Soft-AP association**: init/IP/MAC/count are live, but no virtual
  station exists to associate — `softAPgetStationNum()` stays 0 and the
  DHCP server path is untested.
- **ESP-NOW encrypted peers / broadcast stress / >2 nodes**: untested
  (unencrypted two-node delivery verified).
- **Enterprise WPA2 / WPS / promiscuous / monitor mode**: not modeled.
- **BTDM advertising**: unsupported — the Link Layer/baseband lives in
  ROM and needs RF-hardware emulation (full analysis 2026-08-22);
  controller-init (`test-worker-bt`) remains the supported scope.

## pcap capture

`proxy.getPcapData()` returns a standard big-endian pcap (global header +
per-record headers — previously the records were little-endian, which made
Wireshark/tshark misparse every packet). Validate with
`tshark -r capture.pcap` or `tcpdump -r capture.pcap -nn`. Duplicate
responses at identical timestamps and board-sent ICMP port-unreachables in
captures are normal: LWIP retries queries on its (sim-time) schedule while
answers arrive on wall-clock time, so retries and late answers overlap —
not emulator bugs.

## Timing advisory for network tests (the caveat, fixed)

`runSimChunk()` blocks the worker event loop (and the gateway socket with
it) for a whole sim chunk, and idle phases fast-forward sim time ~1000×
vs wall-clock. Two consequences, both handled in the test:

1. **Delivery quantization**: default 500K-step chunks stall inbound
   delivery ~1s while sim time races ahead, so firmware timeouts expire
   first. Network tests must set a small chunk size
   (`proxy.chunkSize = 20000` **via the setter** — `init()` forwards
   `proxy._chunkSize`, so a config key alone is overwritten).
2. **Poll windows burning**: `delay(500)`-based poll loops idle the CPU,
   so a 40-sim-second window can elapse in wall-ms, before the ~40ms-wall
   reply arrives. UDP wait loops busy-spin instead (keeps sim:wall ≤ ~1).
   All network phases additionally retry DNS/connects like real firmware.

Upstream public servers are occasionally slow/filtered (observed:
`time.cloudflare.com:123` blackholes this host while `time.windows.com`
answers) — the test uses servers verified reachable and retries anyway.

## Test-code lessons (from debugging this battery)

- CoAP header bit layout: version is bits[7:6], type bits[5:4] —
  `(b>>4)==1` / `(b>>2)&3` silently reject every valid ACK. ACK byte is
  `0x60|tkl` (ver=1, type=2), not `0x80` (ver=2/CON).
- `test.mosquitto.org` has no usable IPv4 dependency issues here, but
  prefer servers with stable IPv4 answers; the test resolves with retries.
