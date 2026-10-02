package main

// Unified L2-hub client loop. EVERY board (ESP32, STM32F4, Pico/RP2350)
// speaks the same wire protocol here: binary Ethernet frames in, room
// broadcast + gVisor VN pipe out, plus the "RESET" text control (STM32
// path — also harmless for ESP32/Pico clients).
//
// Per-frame hook order (first claim wins; claim = skip VN pipe AND room
// broadcast):
//   0. "RESET" text control (STM32 cli.mjs --connect rounds; also safe
//      for ESP32/Pico — tears down this room + drops the shared VN).
//   1. snoopVStaRaw  (ESP32 Soft-AP 802.11 medium frames — vsta.go)
//   2. DHCP dport 67 (all boards — dhcp.go; never enters gVisor)
//   3. sendARPReply  (gateway .4.1 ARP, all boards — dhcp.go)
//   4. divertUDPForward (CoAP-style UDP server replies — dhcp.go)
//   5. snoopIPv6     (ESP32 handcrafted IPv6 — ipv6.go)
//      handleICMPv6 + handleNAT64 (Pico NDP/NAT64 — icmpv6.go/nat64.go)
//      Selection rule (see ipv6.go): boards that completed the ESP32 SLAAC
//      handshake (unicast RA via raTargets) stay on the ESP32 path; all
//      other v6 frames use the Pico path. Both paths claim with `continue`.
//   6. ESP-NOW magic E5 50 4E 57 (room broadcast, never gVisor)
// Everything else: VN pipe feed + room broadcast (shared L2 hub).

import (
	"context"
	"encoding/binary"
	"fmt"
	"time"

	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
	"github.com/gorilla/websocket"
)

// handleClient reads Ethernet frames from a specific WebSocket client and multiplexes them.
func handleClient(client *Client, room *Room) {
	defer func() { removeClient(client, room) }()

	// Ping/Pong Timeout Setup
	pongWait := 60 * time.Second
	pingPeriod := 50 * time.Second

	client.Conn.SetReadDeadline(time.Now().Add(pongWait))
	client.Conn.SetPongHandler(func(string) error {
		client.Conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	clientCtx, clientCancel := context.WithCancel(context.Background())
	defer clientCancel()

	go func() {
		ticker := time.NewTicker(pingPeriod)
		defer ticker.Stop()
		for {
			select {
			case <-clientCtx.Done():
				return
			case <-room.Ctx.Done():
				return
			case <-ticker.C:
				client.WriteMutex.Lock()
				err := client.Conn.WriteMessage(websocket.PingMessage, nil)
				client.WriteMutex.Unlock()
				if err != nil {
					client.Conn.Close()
					return
				}
			}
		}
	}()

	for {
		messageType, msg, err := client.Conn.ReadMessage()
		if err != nil {
			fmt.Printf("WebSocket Read Error: %v\n", err)
			return
		}

		// Control message (STM32 cli.mjs --connect mode between firmware
		// rounds): tear down this room and drop the shared gVisor stack
		// so the next connection starts with a clean session table.
		if messageType == websocket.TextMessage {
			if string(msg) == "RESET" {
				fmt.Printf("[Network Gateway] RESET requested for room %s — clearing gVisor session state\n", room.SessionId)
				room.Cancel()
				if room.PipeToVN != nil {
					room.PipeToVN.Close()
				}
				roomsMutex.Lock()
				delete(rooms, room.SessionId)
				roomsMutex.Unlock()
				globalVNMutex.Lock()
				globalVN = nil
				globalVNMutex.Unlock()
				client.Conn.Close()
				return
			}
			continue
		}

		if messageType == websocket.BinaryMessage {
			// (1) Virtual Soft-AP stations FIRST (ESP32 path): EPWF-marked
			// worker-tap 802.11 medium frames + raw py-vsta mgmt frames
			// must never reach the Ethernet log/parse below (their bytes
			// misparse as dst/src/ethertype) nor the VN pipe (gVisor
			// chokes — the pipe is the shared hub<->gVisor ETHERNET
			// channel). snoopVStaRaw claims them (skips pipe AND room
			// broadcast); gateway-originated 802.11 is re-broadcast to
			// room peers via DIRECT WebSocket in vstaDeliverBeacon —
			// never the pipe.
			if snoopVStaRaw(msg, client, room) {
				continue
			}
			if len(msg) >= 14 {
				dst := msg[0:6]
				src := msg[6:12]
				ethType := binary.BigEndian.Uint16(msg[12:14])
				fmt.Printf("[%s] [ESP32 -> Hub] >> Eth Frame (dst=%x, src=%x, type=0x%04x, len=%d)\n", time.Now().Format("15:04:05.000"), dst, src, ethType, len(msg))
			}

			// Intercept DHCP Packets
			packet := gopacket.NewPacket(msg, layers.LayerTypeEthernet, gopacket.Default)
			if arpLayer := packet.Layer(layers.LayerTypeARP); arpLayer != nil {
				arp, _ := arpLayer.(*layers.ARP)
				// Answer ARP for the gateway IP right here: gVisor's cold
				// stack can take ~1s to answer its first ARP, which breaks
				// the board's first reply of a session (it ARPs, hears
				// nothing, and drops the frame).
				if arp.Operation == layers.ARPRequest &&
					(arp.DstProtAddress[0] == 192 && arp.DstProtAddress[1] == 168 &&
						arp.DstProtAddress[2] == 4 && arp.DstProtAddress[3] == 1) {
					sendARPReply(client, arp)
					continue
				}
			}
			if udpLayer := packet.Layer(layers.LayerTypeUDP); udpLayer != nil {
				udp, _ := udpLayer.(*layers.UDP)
				if udp.DstPort == 67 {
					handleDHCP(msg, packet, client, room)
					continue // DO NOT forward to gVisor or other clients!
				}
			}

			// UDP-forward replies: board -> 192.168.4.1:<alloc> are relayed
			// to the mapped host client, never entering gVisor (nothing
			// listens for them there). See dhcp.go handleUDPProxy.
			if divertUDPForward(packet, room) {
				continue
			}

			// IPv6 gateway services. TWO realms share ethertype 0x86DD:
			// ESP32 (snoopIPv6 — fd00::1 + EUI-64 link-local, unicast RA
			// for its LWIP multicast filter) and Pico (handleICMPv6 +
			// handleNAT64 — fe80::1, fd00:4::1 incl. RDNSS, WKP NAT64).
			// snoopIPv6 claims ONLY ESP32-realm destinations (its RS case
			// answers multicast solicits with a unicast RA, which is how
			// the ESP32 path learns boards); everything else falls to the
			// Pico path, then to room broadcast + gVisor like before.
			if snoopIPv6(msg, client, room) {
				continue
			}
			if len(msg) >= 14 && binary.BigEndian.Uint16(msg[12:14]) == 0x86DD {
				if handleICMPv6(msg, client) {
					continue
				}
				if handleNAT64(msg, client, room) {
					continue
				}
			}

			// ESP-NOW medium frames (magic-prefixed raw 802.11): broadcast
			// to room peers but NEVER feed gVisor (it would choke on the
			// 802.11 bytes as Ethernet). Peers strip the magic on receipt.
			isEspNow := len(msg) > 4 && msg[0] == 0xE5 && msg[1] == 0x50 && msg[2] == 0x4E && msg[3] == 0x57
			if !isEspNow {
				// 1. Send frame to gVisor stack
				if err := feedVN(room, msg); err != nil {
					fmt.Printf("[Room %s] Pipe Write Error: %v\n", room.SessionId, err)
					return
				}
			}

			// 2. Broadcast frame to all *other* clients (Layer 2 Hub logic)
			broadcastOthers(room, client, msg)
		}
	}
}
