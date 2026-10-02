package main

// openhw-unified-gateway: ONE binary serving ESP32 + STM32F4 + Pico/RP2350
// boards with every feature of the three legacy trees:
//
//   shared core (room.go/hub.go/dhcp.go/bridge.go — identical in all trees)
//   + ESP32 extras : vsta.go (virtual Soft-AP), handleIPv6.go (SLAAC/echo),
//     ARP fast-reply + UDP-forward + ESP-NOW hub hooks (in hub.go)
//   + Pico extras  : handleICMPv6.go (NDP/echo), nat64.go (NAT64/DNS64),
//     Room.NAT64 wiring + raTickerLoop (in hub/main wiring)
//   + STM32 extras : TLS additive listener, createConfig() VN profile,
//     globalVNMutex discipline, "RESET" text control (in hub.go)
//
// Defaults: listen 127.0.0.1:5030 (GATEWAY_PORT overrides), same
// /api/network-gateway + /api/ble-gateway + /api/thread-gateway routes,
// BLE dials 127.0.0.1:9544. GATEWAY_MODE=public keeps the legacy meaning.

import (
	"context"
	"crypto/rand"
	"crypto/tls"
	"fmt"
	"net"
	"net/http"

	"github.com/containers/gvisor-tap-vsock/pkg/types"
	"github.com/containers/gvisor-tap-vsock/pkg/virtualnetwork"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins for the local gateway
	},
}

func main() {
	parseFlags()

	fmt.Println("===================================================")
	fmt.Println("   OpenHW Studio - Unified IoT Gateway (Go)")
	fmt.Println("   ESP32 + STM32F4 + Pico/RP2350 — one binary, port 5030")
	fmt.Println("===================================================")

	http.HandleFunc("/api/network-gateway", func(w http.ResponseWriter, r *http.Request) {
		bridgeMutex.Lock()
		if bridgeEnabled {
			if bridgeTAP == nil {
				bt, err := createBridgeTAP(context.Background())
				if err != nil {
					bridgeMutex.Unlock()
					fmt.Printf("[Bridge] Failed to create TAP: %v\n", err)
					http.Error(w, "Failed to create TAP interface. On Windows, install the TAP driver from OpenVPN.", http.StatusInternalServerError)
					return
				}
				bridgeTAP = bt
			}
			bt := bridgeTAP
			bridgeMutex.Unlock()

			wsConn, err := upgrader.Upgrade(w, r, nil)
			if err != nil {
				fmt.Printf("[Bridge] WS upgrade error: %v\n", err)
				return
			}

			client := &Client{Conn: wsConn}
			bt.mutex.Lock()
			bt.clients[client] = true
			bt.mutex.Unlock()

			handleBridgeClient(client, bt)
			return
		}
		bridgeMutex.Unlock()

		sessionId := r.URL.Query().Get("sessionId")
		if sessionId == "" {
			b := make([]byte, 8)
			_, _ = rand.Read(b)
			sessionId = fmt.Sprintf("isolated-%x", b)
		}

		roomsMutex.Lock()
		room, exists := rooms[sessionId]
		if !exists {
			fmt.Printf("[Network Gateway] Creating new Virtual Network room: %s\n", sessionId)

			var currentVN *virtualnetwork.VirtualNetwork

			cfg := createConfig()
			if !isPublicMode() {
				globalVNMutex.RLock()
				haveGlobal := globalVN != nil
				globalVNMutex.RUnlock()
				if !haveGlobal {
					vn, err := virtualnetwork.New(&cfg)
					if err != nil {
						roomsMutex.Unlock()
						fmt.Printf("Error creating virtual network: %v\n", err)
						return
					}
					globalVNMutex.Lock()
					globalVN = vn
					globalVNMutex.Unlock()
				}
				globalVNMutex.RLock()
				currentVN = globalVN
				globalVNMutex.RUnlock()
			} else {
				vn, err := virtualnetwork.New(&cfg)
				if err != nil {
					roomsMutex.Unlock()
					fmt.Printf("Error creating virtual network: %v\n", err)
					return
				}
				currentVN = vn
			}

			pipe1, pipe2, err := connLoopback()
			if err != nil {
				roomsMutex.Unlock()
				fmt.Printf("Error creating pipe: %v\n", err)
				return
			}

			ctx, cancel := context.WithCancel(context.Background())

			room = &Room{
				SessionId: sessionId,
				VN:        currentVN,
				Clients:   make(map[*Client]bool),
				PipeToVN:  pipe2,
				Ctx:       ctx,
				Cancel:    cancel,
				NextIP:    2,
				MacToIP:   make(map[string]net.IP),
			}
			rooms[sessionId] = room

			// Per-room NAT64 engine (Pico path; room-scoped, dies with ctx).
			room.NAT64 = newNat64Engine(room.Ctx.Done())
			go room.NAT64.sweepLoop()

			// Start the single gVisor acceptor for this room
			go currentVN.AcceptQemu(ctx, pipe1)

			// Start the single reader that takes frames from gVisor and broadcasts to ALL clients
			go gvisorToClientsLoop(room)

			// Periodic unsolicited RAs (Pico path: all-nodes, for
			// timer-less stacks; ESP32 path runs its own unicast ticker).
			go raTickerLoop(room)

		} else {
			fmt.Printf("[Network Gateway] Joining existing Virtual Network room: %s\n", sessionId)
		}
		roomsMutex.Unlock()

		fmt.Printf("\n--- INCOMING WEBSOCKET REQUEST ---\n")
		fmt.Printf("URL: %s\n", r.URL.String())

		wsConn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			fmt.Printf("WebSocket Upgrade Error: %v\n", err)
			return
		}

		fmt.Println("[Network Gateway] Client connected. Upgrade successful!")

		client := &Client{Conn: wsConn}
		room.Lock()
		room.Clients[client] = true
		room.Unlock()

		handleClient(client, room)
	})

	port := gatewayPort()
	listenAddr := "127.0.0.1:" + port
	if isPublicMode() {
		listenAddr = ":" + port
	}

	// Start terminal command loop for bridge mode control
	go startCommandLoop()

	// Periodic IPv6 Router Advertisements (ESP32 unicast path).
	go startPeriodicRA()

	http.HandleFunc("/api/ble-gateway", handleBLEGateway)
	http.HandleFunc("/api/thread-gateway", handleThreadGateway)

	fmt.Printf("[Network Gateway] Server running on ws://%s/api/network-gateway\n", listenAddr)

	// TLS flags (STM32 path): serve the SAME mux over HTTPS/WSS as well,
	// so an https:// page (e.g. GitHub Pages) can connect with wss://.
	// Plain ws:// keeps working on listenAddr; the TLS listener is
	// additive, never a replacement.
	tlsPort, tlsCert, tlsKey := tlsConfig()
	if tlsPort != "" {
		if tlsCert == "" || tlsKey == "" {
			fmt.Printf("[Network Gateway] TLS requested (port %s) but TLS_CERT/TLS_KEY (or --tls-cert/--tls-key) missing — WSS disabled\n", tlsPort)
		} else if _, err := tls.LoadX509KeyPair(tlsCert, tlsKey); err != nil {
			fmt.Printf("[Network Gateway] WSS disabled: cannot load cert/key (%v)\n", err)
		} else {
			tlsAddr := "127.0.0.1:" + tlsPort
			fmt.Printf("[Network Gateway] Server also running on wss://%s/api/network-gateway\n", tlsAddr)
			go func() {
				if err := http.ListenAndServeTLS(tlsAddr, tlsCert, tlsKey, nil); err != nil {
					fmt.Printf("WSS Server Error: %v\n", err)
				}
			}()
		}
	}
	if err := http.ListenAndServe(listenAddr, nil); err != nil {
		fmt.Printf("Server Error: %v\n", err)
	}
}

// createConfig is the shared gVisor VN profile (STM32 tree; the ESP32 and
// Pico trees use the identical subnet/gateway/MAC/MTU — only the STM32
// tree factored it into a function, and it carries the NAT map).
func createConfig() types.Configuration {
	return types.Configuration{
		Debug:             false,
		MTU:               1500,
		Subnet:            "192.168.4.0/24",
		GatewayIP:         "192.168.4.1",
		GatewayMacAddress: "5a:94:ef:e4:0c:dd",
		NAT:               map[string]string{"10.150.211.85": "127.0.0.1"},
	}
}
