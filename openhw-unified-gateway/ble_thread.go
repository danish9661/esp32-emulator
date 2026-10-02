package main

import (
	"fmt"
	"net"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

// handleBLEGateway proxies BLE HCI packets between browser WS and Bumble TCP
func handleBLEGateway(w http.ResponseWriter, r *http.Request) {
	fmt.Printf("[BLE Gateway] Incoming connection request from %s\n", r.RemoteAddr)
	wsConn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		fmt.Printf("[BLE] WS upgrade error: %v\n", err)
		return
	}
	defer wsConn.Close()

	fmt.Println("[BLE Gateway] Browser client connected. Dialing Bumble TCP server (localhost:9544)...")
	tcpConn, err := net.DialTimeout("tcp", "127.0.0.1:9544", 5*time.Second)
	if err != nil {
		fmt.Printf("[BLE Gateway] Failed to connect to Bumble (port 9544): %v. Please make sure vhci_bridge.py is running.\n", err)
		wsConn.WriteMessage(websocket.TextMessage, []byte("ERROR: Failed to connect to Bumble (port 9544). Ensure vhci_bridge.py is running."))
		return
	}
	defer tcpConn.Close()
	fmt.Println("[BLE Gateway] Connected to Bumble TCP server successfully!")

	errChan := make(chan error, 2)

	// WS -> TCP
	go func() {
		for {
			messageType, msg, err := wsConn.ReadMessage()
			if err != nil {
				errChan <- err
				return
			}
			if messageType == websocket.BinaryMessage {
				_, err = tcpConn.Write(msg)
				if err != nil {
					errChan <- err
					return
				}
			}
		}
	}()

	// TCP -> WS
	go func() {
		buf := make([]byte, 4096)
		for {
			n, err := tcpConn.Read(buf)
			if err != nil {
				errChan <- err
				return
			}
			if n > 0 {
				err = wsConn.WriteMessage(websocket.BinaryMessage, buf[:n])
				if err != nil {
					errChan <- err
					return
				}
			}
		}
	}()

	err = <-errChan
	fmt.Printf("[BLE Gateway] Connection closed: %v\n", err)
}

type ThreadClient struct {
	Conn       *websocket.Conn
	WriteMutex sync.Mutex
}

type ThreadRoom struct {
	sync.Mutex
	SessionId string
	Clients   map[*ThreadClient]bool
}

var (
	threadRoomsMutex sync.Mutex
	threadRooms      = make(map[string]*ThreadRoom)
)

// handleThreadGateway routes IEEE 802.15.4 frames between C6 devices in the same session
func handleThreadGateway(w http.ResponseWriter, r *http.Request) {
	sessionId := r.URL.Query().Get("sessionId")
	if sessionId == "" {
		sessionId = "default"
	}

	wsConn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		fmt.Printf("[Thread] WS upgrade error: %v\n", err)
		return
	}

	client := &ThreadClient{Conn: wsConn}

	threadRoomsMutex.Lock()
	room, exists := threadRooms[sessionId]
	if !exists {
		room = &ThreadRoom{
			SessionId: sessionId,
			Clients:   make(map[*ThreadClient]bool),
		}
		threadRooms[sessionId] = room
		fmt.Printf("[Thread Gateway] Created new room: %s\n", sessionId)
	}
	room.Lock()
	room.Clients[client] = true
	room.Unlock()
	threadRoomsMutex.Unlock()

	fmt.Printf("[Thread Gateway] Client joined room %s\n", sessionId)

	defer func() {
		wsConn.Close()
		room.Lock()
		delete(room.Clients, client)
		isEmpty := len(room.Clients) == 0
		room.Unlock()

		if isEmpty {
			threadRoomsMutex.Lock()
			delete(threadRooms, sessionId)
			threadRoomsMutex.Unlock()
			fmt.Printf("[Thread Gateway] Cleaned up room %s\n", sessionId)
		}
	}()

	for {
		messageType, msg, err := wsConn.ReadMessage()
		if err != nil {
			return
		}

		if messageType == websocket.BinaryMessage {
			room.Lock()
			targets := make([]*ThreadClient, 0, len(room.Clients))
			for other := range room.Clients {
				if other != client {
					targets = append(targets, other)
				}
			}
			room.Unlock()

			for _, other := range targets {
				other.WriteMutex.Lock()
				other.Conn.WriteMessage(websocket.BinaryMessage, msg)
				other.WriteMutex.Unlock()
			}
		}
	}
}
