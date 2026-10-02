package main

// Shared room/hub core: ONE gateway binary serves ESP32 + STM32F4 +
// Pico/RP2350 boards in the same room model. Every board speaks the same
// L2-hub protocol (binary Ethernet frames over ws, BOARD_IP/PORT_FORWARD
// text announcements, sessionId rooms), so rooms, the gVisor VN, the
// VN pipe, and the two fan-out/fan-in loops are board-agnostic.
//
// Board-specific frame hooks plug into handleClient (hub.go) in a fixed
// order; each hook returns true when it consumed the frame (caller must
// then skip the VN pipe AND the room broadcast for it):
//   1. vstaSnoop     (ESP32 Soft-AP 802.11 medium frames — vsta.go)
//   2. udpForwardDivert (CoAP-style UDP server replies — dhcp.go)
//   3. ipv6Snoop     (ESP32 handcrafted IPv6 — ipv6.go; Pico NAT64 — nat64.go)
//   4. espNowisiones  (ESP-NOW magic frames — hub.go)
// Everything else flows to the VN pipe + room broadcast as before.

import (
	"context"
	"encoding/binary"
	"fmt"
	"io"
	"net"
	"sync"
	"time"

	"github.com/containers/gvisor-tap-vsock/pkg/virtualnetwork"
	"github.com/gorilla/websocket"
)

// Client is one WebSocket connection (any board).
type Client struct {
	Conn       *websocket.Conn
	WriteMutex sync.Mutex
}

// Room is one L2 hub segment: a set of board connections sharing a gVisor
// VN plus an optional per-room NAT64 engine (Pico path).
type Room struct {
	sync.Mutex
	SessionId string
	VN        *virtualnetwork.VirtualNetwork
	Clients   map[*Client]bool
	PipeToVN  net.Conn
	Ctx       context.Context
	Cancel    context.CancelFunc
	NextIP    byte
	MacToIP   map[string]net.IP
	NAT64     *Nat64Engine
}

var (
	roomsMutex sync.Mutex
	rooms      = make(map[string]*Room)
	globalVN   *virtualnetwork.VirtualNetwork
	// globalVNMutex serializes lazy creation/teardown of the shared VN
	// (STM32 path RESET handler + proxy dial path). The ESP32 path grew
	// without it (roomsMutex only); keep the mutex here so both paths
	// share one discipline.
	globalVNMutex sync.RWMutex
)

func connLoopback() (net.Conn, net.Conn, error) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return nil, nil, err
	}
	port := listener.Addr().(*net.TCPAddr).Port
	conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		return nil, nil, err
	}
	conn2, err := listener.Accept()
	listener.Close()
	return conn, conn2, err
}

// gvisorToClientsLoop reads Ethernet frames from gVisor and broadcasts them to all connected clients in the room.
func gvisorToClientsLoop(room *Room) {
	for {
		var length uint32
		err := binary.Read(room.PipeToVN, binary.BigEndian, &length)
		if err != nil {
			fmt.Printf("[Room %s] Pipe Read Error (size): %v\n", room.SessionId, err)
			return
		}

		buf := make([]byte, length)
		_, err = io.ReadFull(room.PipeToVN, buf)
		if err != nil {
			fmt.Printf("[Room %s] Pipe Read Error (data): %v\n", room.SessionId, err)
			return
		}

		if len(buf) >= 14 {
			dst := buf[0:6]
			src := buf[6:12]
			ethType := binary.BigEndian.Uint16(buf[12:14])
			fmt.Printf("[%s] [gVisor -> Hub] << Eth Frame (dst=%x, src=%x, type=0x%04x, len=%d)\n", time.Now().Format("15:04:05.000"), dst, src, ethType, length)
		}

		// Broadcast frame to all clients
		room.Lock()
		targets := make([]*Client, 0, len(room.Clients))
		for client := range room.Clients {
			targets = append(targets, client)
		}
		room.Unlock()

		for _, client := range targets {
			client.WriteMutex.Lock()
			err = client.Conn.WriteMessage(websocket.BinaryMessage, buf)
			client.WriteMutex.Unlock()
			if err != nil {
				fmt.Printf("[Room %s] Client Write Error: %v\n", room.SessionId, err)
			}
		}
	}
}

// broadcastOthers relays one frame to every room peer except the sender.
func broadcastOthers(room *Room, sender *Client, msg []byte) {
	room.Lock()
	targets := make([]*Client, 0, len(room.Clients))
	for otherClient := range room.Clients {
		if otherClient != sender {
			targets = append(targets, otherClient)
		}
	}
	room.Unlock()

	for _, otherClient := range targets {
		otherClient.WriteMutex.Lock()
		otherClient.Conn.WriteMessage(websocket.BinaryMessage, msg)
		otherClient.WriteMutex.Unlock()
	}
}

// feedVN writes one frame length-prefixed onto the room pipe (serialized
// by vnPipeMu — see dhcp.go).
func feedVN(room *Room, msg []byte) error {
	room.Lock()
	pipe := room.PipeToVN
	room.Unlock()

	if pipe == nil {
		return fmt.Errorf("room pipe gone")
	}
	vnPipeMu.Lock()
	defer vnPipeMu.Unlock()
	if err := binary.Write(pipe, binary.BigEndian, uint32(len(msg))); err != nil {
		return err
	}
	_, err := pipe.Write(msg)
	return err
}

// removeClient unlinks a client; when the room empties it tears down the
// VN session (per-room VN in public mode is room-owned; the shared global
// VN survives — only the room mapping is dropped).
func removeClient(client *Client, room *Room) {
	client.Conn.Close()
	room.Lock()
	delete(room.Clients, client)
	isEmpty := len(room.Clients) == 0
	room.Unlock()
	fmt.Println("[Network Gateway] Client disconnected.")

	if isEmpty {
		fmt.Printf("[Room %s] Empty! Cleaning up virtual network...\n", room.SessionId)
		room.Cancel() // Stop vn.AcceptQemu
		if room.PipeToVN != nil {
			room.PipeToVN.Close() // Stop gvisorToClientsLoop
		}

		roomsMutex.Lock()
		delete(rooms, room.SessionId)
		roomsMutex.Unlock()
	}
}
