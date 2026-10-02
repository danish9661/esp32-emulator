# openhw-unified-gateway

ONE gateway binary for all three OpenHW emulator boards — ESP32, STM32F4,
Pico/RP2350 — with every feature of the three legacy trees merged:

| Feature | ESP32 tree | STM32 tree | Pico tree | Unified |
|---|---|---|---|---|
| L2 hub rooms + gVisor VN + DHCP + TCP forward | yes | yes | yes | yes |
| UDP forward (CoAP-style `:5683`) | yes | — | — | yes |
| ARP fast-reply for `.4.1` | yes | — | — | yes |
| Virtual Soft-AP stations (vsta) | yes | — | — | yes |
| Handcrafted IPv6 (`fd00::1` SLAAC/echo) | yes | — | — | yes (ESP32 realm) |
| ESP-NOW hub broadcast | yes | — | — | yes |
| NDP/ICMPv6 (`fe80::1`, `fd00:4::1` + RDNSS) | — | — | yes | yes (Pico realm) |
| NAT64/DNS64 (`64:ff9b::/96`) | — | — | yes | yes |
| TLS additive listener | — | yes | — | yes |
| VN `createConfig()` profile + NAT map | — | yes | — | yes |
| `globalVNMutex` discipline | — | yes | — | yes |
| `RESET` text control | — | yes | — | yes |
| BLE `/api/ble-gateway` (`:9544`) | yes | yes | yes | yes |
| Thread `/api/thread-gateway` | yes | yes | yes | yes |
| TAP bridge mode | yes | yes | yes | yes |

## Running it

One binary serves all three boards at once — start it a single time:

```bash
cd openhw-unified-gateway
go build -o openhw-gw .     # or ./start-gateway.sh
./openhw-gw                 # listens on 127.0.0.1:5030
```

Point every board at the same URL (default port **5030**):

- ESP32 emulator: `ws://127.0.0.1:5030/api/network-gateway`
- STM32F4 (`cli.mjs`): `--gw-url ws://127.0.0.1:5030/api/network-gateway`
- Pico (`cli.js` / web): `--gateway ws://127.0.0.1:5030/api/network-gateway`

(`GATEWAY_PORT` overrides the port; `GATEWAY_MODE=public` keeps the legacy
meaning — per-room VN, public bind, no port forwards. TLS via `TLS_PORT` /
`TLS_CERT` / `TLS_KEY` or `--tls-port/--tls-cert/--tls-key`.)

## How the merge works

- Shared core (`room.go`, `hub.go`, `dhcp.go`, `bridge.go`,
  `ble_thread.go`): rooms, gVisor VN, L2 hub loop, DHCP + TCP/UDP
  forwards, TAP bridge, BLE/Thread gateways — byte-identical logic to the
  legacy trees.
- ESP32 extras (`vsta.go`, `ipv6.go`, hub hooks): virtual Soft-AP
  stations, handcrafted IPv6, ARP fast-reply, UDP-forward, ESP-NOW.
- Pico extras (`icmpv6.go`, `nat64.go` + tests): NDP/echo, NAT64/DNS64.
  IPv6 realms are disjoint by design: ESP32 serves `fd00::1` (+ EUI-64
  link-local), Pico serves `fe80::1` / `fd00:4::1` / WKP — `snoopIPv6`
  claims only ESP32-realm destinations, the rest falls to the Pico path.
- STM32 extras (`flags.go`, `main.go`, hub RESET): TLS listener,
  `createConfig()` VN profile, `globalVNMutex`, `RESET` control.

## Testing

```bash
go vet ./...     # clean
go test ./...    # Pico NAT64/ICMPv6 suite (13 tests) + build check
```

Board-level verification (needs each emulator repo + its firmware):
DHCP + `BOARD_IP` on all three boards in one shared room, ESP32 WiFi scan,
STM32 ethernet ping, Pico WiFi + `ping6`, CoAP via UDP forward, Soft-AP
station count, ESP-NOW A↔B.

## Legacy trees (deprecated)

The three per-board trees are frozen — do not extend them:

- `openhw-studio-gateway/` (this repo, ESP32 history, `:5085`)
- `stm32 F4/openhw-local-gateway/` (`:5070`)
- `rp2350/Pico-emu/openhw-studio-gateway/` (`:5090`)

All new gateway work lands here. The legacy binaries keep working (their
ports are unchanged), but every board also works against this unified
binary on `:5030`, and CI/docs point here.
