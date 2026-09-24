package main

import (
	"encoding/binary"
	"fmt"
	"net"
	"sync"
	"time"

	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
	"github.com/gorilla/websocket"
	"github.com/insomniacslk/dhcp/dhcpv4"
)

// ---- Virtual Soft-AP stations -------------------------------------------
// A Soft-AP board (the ESP32 running WiFi.softAP()) has no gateway-side
// presence of its own: the board IS the AP. Real stations, however, need
// somewhere to associate. This file implements that somewhere: gateway-side
// virtual stations that speak just enough 802.11 + DHCP for the board's own
// AP stack to believe a client joined:
//
//   probe req -> probe resp | auth -> auth resp | assoc req -> assoc resp
//   -> DHCP discover -> offer -> request -> ACK -> ICMP echo + UDP echo
//
// Design rules (earned the hard way elsewhere in this gateway):
//   * Everything is per-room, keyed like the DHCP tables. Rooms are
//     per-connection, so global state would leak clients across dead rooms.
//   * Gateway-originated 802.11 (beacon copies, probe/auth/assoc
//     responses) goes via DIRECT WebSocket broadcast to room peers
//     (vstaDeliverBeacon) — never the room pipe. The pipe is the shared
//     hub<->gVisor Ethernet channel (QEMU protocol); 802.11 bytes there
//     would choke gVisor. The worker and py-vsta observers both receive
//     gateway 802.11 as WS binary messages.
//   * DHCP for AP clients reuses handleDHCP's tables (globalMacToIP /
//     globalIPToPort / udpFwdListeners) so port/UDP forwards keep working.
//   * 802.11 sequence numbers advance per transmitted frame (12-bit, in
//     seq-ctrl bits 4..15), like the engine's own builders.
//
// Trigger: the Soft-AP board emits beacons + AP-side responses (probe/
// auth/assoc responses, data) through the worker EPWF tap; external
// stations (py-vsta proof, second boards) send raw 802.11 management
// addressed to the AP under test. The hub loop (main.go) calls
// snoopVStaRaw() before the gVisor pipe; when it claims a frame it must
// skip the VN pipe AND the room broadcast (responses are re-broadcast to
// peers explicitly where observers need them).

var (
	vstaMu     sync.Mutex
	vstaByRoom = make(map[*Room]*vstaRoom)
	// AP-client pool starts at .100 (station DHCP pool uses .2+).
	vstaNextIP byte = 100
	vstaSeq    uint32
)

type vstaClient struct {
	mac      net.HardwareAddr
	ip       net.IP
	last     time.Time
	assoc    bool
	dhcpDone bool
}

type vstaRoom struct {
	// apMAC/apSSID learned from the board's own beacons (the AP under
	// test). First beacon wins; never flap mid-session.
	apMAC  net.HardwareAddr
	apSSID string
	// staMAC is OUR virtual station identity on this room (one is
	// enough: the board counts it via esp_wifi_ap_get_sta_list).
	staMAC  net.HardwareAddr
	clients map[string]*vstaClient // keyed by MAC string
}

func vstaRoomFor(room *Room) *vstaRoom {
	vr, ok := vstaByRoom[room]
	if !ok || vr == nil {
		vr = &vstaRoom{clients: make(map[string]*vstaClient)}
		// Deterministic station MAC per room (OUI 02:56:53 = "VST"
		// with the local bit; low byte derived from the session id so
		// two rooms never collide on a shared capture).
		h := 0
		for _, c := range room.SessionId {
			h = (h*31 + int(c)) & 0xFF
		}
		if h < 3 {
			h += 3
		}
		vr.staMAC = net.HardwareAddr{0x02, 0x56, 0x53, 0x54, 0x41, byte(h)}
		vstaByRoom[room] = vr
	}
	return vr
}

func vstaNextSeqCtrl() uint16 {
	vstaMu.Lock()
	vstaSeq = (vstaSeq + 1) % 4096
	s := vstaSeq
	vstaMu.Unlock()
	return uint16(s<<4) & 0xFFF0
}

func vstaFrameHeader(fc0, fc1 byte, da, sa, bssid net.HardwareAddr, seqCtrl uint16) []byte {
	h := make([]byte, 24)
	h[0] = fc0
	h[1] = fc1
	h[2], h[3] = 0, 0
	copy(h[4:10], da)
	copy(h[10:16], sa)
	copy(h[16:22], bssid)
	binary.LittleEndian.PutUint16(h[22:24], seqCtrl)
	return h
}

// Probe response advertising the AP's own SSID (so the board's scan finds
// its own network even with no other client in the room).
func (vr *vstaRoom) buildProbeResp(staMAC net.HardwareAddr) []byte {
	h := vstaFrameHeader(0x50, 0x00, staMAC, vr.apMAC, vr.apMAC, vstaNextSeqCtrl())
	body := []byte{0, 0, 0, 0, 0, 0, 0, 0, 0x64, 0x00, 0x01, 0x04}
	body = append(body, 0x00, byte(len(vr.apSSID)))
	body = append(body, vr.apSSID...)
	body = append(body, 0x01, 0x08, 0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24)
	body = append(body, 0x03, 0x01, 0x06)
	return append(h, body...)
}

func (vr *vstaRoom) buildAuthResp(staMAC net.HardwareAddr) []byte {
	h := vstaFrameHeader(0xB0, 0x00, staMAC, vr.apMAC, vr.apMAC, vstaNextSeqCtrl())
	return append(h, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00)
}

func (vr *vstaRoom) buildAssocResp(staMAC net.HardwareAddr, aid uint16) []byte {
	h := vstaFrameHeader(0x10, 0x00, staMAC, vr.apMAC, vr.apMAC, vstaNextSeqCtrl())
	body := []byte{0x11, 0x04, 0x00, 0x00, byte(aid & 0xFF), 0xC0}
	body = append(body, 0x01, 0x08, 0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24)
	return append(h, body...)
}

// ---- AP snooping: learn the board's own BSSID/SSID from its beacons ------
// The Soft-AP board announces itself on the room pipe; parse SSID IE 0 +
// DS param IE 3 like the engine's own probe-response parser does.

func noteVStaBeacon(room *Room, msg []byte) {
	if len(msg) < 38 {
		return
	}
	// Management beacon (type 0, subtype 8), broadcast destination.
	if msg[0] != 0x80 {
		return
	}
	bssid := net.HardwareAddr(append([]byte(nil), msg[16:22]...))
	// Never learn the gateway's own AP BSSID as the AP under test.
	if bssid.String() == gwMAC.String() {
		return
	}
	ssid, _ := vstaParseSSIDIE(msg, 36)
	if ssid == "" {
		return
	}
	vstaMu.Lock()
	vr := vstaRoomFor(room)
	if len(vr.apMAC) == 0 {
		vr.apMAC = bssid
		vr.apSSID = ssid
		fmt.Printf("[VSta] AP under test: BSSID=%s SSID=%q\n", bssid.String(), ssid)
	}
	vstaMu.Unlock()
}

func vstaParseSSIDIE(msg []byte, bodyOff int) (string, int) {
	if bodyOff+2 > len(msg) {
		return "", 0
	}
	pos := bodyOff
	channel := 0
	ssid := ""
	for pos+2 <= len(msg) {
		id, ln := msg[pos], int(msg[pos+1])
		if pos+2+ln > len(msg) {
			break
		}
		if id == 0 && ln > 0 {
			ssid = string(msg[pos+2 : pos+2+ln])
		}
		if id == 3 && ln >= 1 {
			channel = int(msg[pos+2])
		}
		pos += 2 + ln
	}
	_ = channel
	return ssid, channel
}

// ---- DHCP for AP clients (gateway answers as the AP's DHCP server) -------
// The board's own DHCPS would normally serve 192.168.4.x, but its server
// only sees frames the engine delivers to it — and engine delivery of
// broadcast DHCP from an unknown station is exactly what we cannot rely
// on. So the gateway answers DHCP for the virtual-station MAC directly,
// leasing from the .100+ pool and reporting the same BOARD_IP /
// PORT_FORWARD / UDP_FORWARD structured messages the station path uses.

func vstaLeaseIP(mac net.HardwareAddr) net.IP {
	macStr := mac.String()
	dhcpMutex.Lock()
	defer dhcpMutex.Unlock()
	if ip, ok := globalMacToIP[macStr]; ok {
		return ip
	}
	// .100+ pool for AP clients (never collides with the .2+ station pool
	// unless >150 stations join — acceptable, mirrors room.NextIP wrap).
	for i := 0; i < 150; i++ {
		ip := net.IPv4(192, 168, 4, vstaNextIP)
		vstaNextIP++
		if vstaNextIP < 100 {
			vstaNextIP = 100
		}
		taken := false
		for _, have := range globalMacToIP {
			if have.Equal(ip) {
				taken = true
				break
			}
		}
		if !taken {
			globalMacToIP[macStr] = ip
			return ip
		}
	}
	return net.IPv4(192, 168, 4, 254)
}

// vstaAnswerDHCP parses the DHCP payload from a virtual-station MAC and
// answers Offer/ACK from the .100+ pool as the AP's DHCP server (server
// identifier = the AP IP 192.168.4.1 so any board-side relay stays sane).
func vstaAnswerDHCP(msg []byte, packet gopacket.Packet, client *Client, room *Room) {
	udpLayer := packet.Layer(layers.LayerTypeUDP)
	ethLayer := packet.Layer(layers.LayerTypeEthernet)
	if udpLayer == nil || ethLayer == nil {
		return
	}
	udp, _ := udpLayer.(*layers.UDP)
	eth, _ := ethLayer.(*layers.Ethernet)
	dhcpPacket, err := dhcpv4.FromBytes(udp.Payload)
	if err != nil {
		fmt.Printf("[VSta] DHCP parse error: %v\n", err)
		return
	}
	macStr := eth.SrcMAC.String()
	ip := vstaLeaseIP(eth.SrcMAC)
	ipStr := ip.String()
	vstaEnsureForwards(client, ipStr, room, macStr)
	vstaSendBoardText(client, "BOARD_IP:"+ipStr)
	vstaMarkDHCP(room, eth.SrcMAC, ip)

	serverIP := net.IPv4(192, 168, 4, 1)
	var replyDHCP *dhcpv4.DHCPv4
	msgType := dhcpPacket.MessageType()
	if msgType == dhcpv4.MessageTypeDiscover {
		fmt.Printf("[VSta] DHCP DISCOVER from %s -> offer %s\n", macStr, ipStr)
		replyDHCP, _ = dhcpv4.NewReplyFromRequest(dhcpPacket,
			dhcpv4.WithMessageType(dhcpv4.MessageTypeOffer),
			dhcpv4.WithYourIP(ip),
			dhcpv4.WithServerIP(serverIP),
			dhcpv4.WithOption(dhcpv4.OptServerIdentifier(serverIP)),
			dhcpv4.WithRouter(serverIP),
			dhcpv4.WithDNS(net.IPv4(8, 8, 8, 8)),
			dhcpv4.WithNetmask(net.IPv4Mask(255, 255, 255, 0)),
			dhcpv4.WithLeaseTime(86400),
		)
	} else if msgType == dhcpv4.MessageTypeRequest {
		fmt.Printf("[VSta] DHCP REQUEST from %s -> ACK %s\n", macStr, ipStr)
		replyDHCP, _ = dhcpv4.NewReplyFromRequest(dhcpPacket,
			dhcpv4.WithMessageType(dhcpv4.MessageTypeAck),
			dhcpv4.WithYourIP(ip),
			dhcpv4.WithServerIP(serverIP),
			dhcpv4.WithOption(dhcpv4.OptServerIdentifier(serverIP)),
			dhcpv4.WithRouter(serverIP),
			dhcpv4.WithDNS(net.IPv4(8, 8, 8, 8)),
			dhcpv4.WithNetmask(net.IPv4Mask(255, 255, 255, 0)),
			dhcpv4.WithLeaseTime(86400),
		)
	} else {
		return
	}

	ethReply := &layers.Ethernet{
		SrcMAC:       net.HardwareAddr{0x5a, 0x94, 0xef, 0xe4, 0x0c, 0xdd},
		DstMAC:       net.HardwareAddr{0xff, 0xff, 0xff, 0xff, 0xff, 0xff}, // Broadcast MAC for DHCP Reply
		EthernetType: layers.EthernetTypeIPv4,
	}
	ipv4Reply := &layers.IPv4{
		Version:  4,
		IHL:      5,
		TTL:      64,
		Protocol: layers.IPProtocolUDP,
		SrcIP:    serverIP,
		DstIP:    net.IPv4(255, 255, 255, 255), // Broadcast IP
	}
	udpReply := &layers.UDP{SrcPort: 67, DstPort: 68}
	udpReply.SetNetworkLayerForChecksum(ipv4Reply)
	buffer := gopacket.NewSerializeBuffer()
	options := gopacket.SerializeOptions{ComputeChecksums: true, FixLengths: true}
	gopacket.SerializeLayers(buffer, options, ethReply, ipv4Reply, udpReply,
		gopacket.Payload(replyDHCP.ToBytes()))
	_ = msg
	client.WriteMutex.Lock()
	client.Conn.WriteMessage(websocket.BinaryMessage, buffer.Bytes())
	client.WriteMutex.Unlock()
}

// ---- per-IP port/UDP forwards (shared with the station path) -------------
// handleDHCP only creates forwards inside its own lease branch; AP clients
// need the same. Lookup-or-create here so both paths share it.

func vstaEnsureForwards(client *Client, ipStr string, room *Room, macString string) {
	dhcpMutex.Lock()
	_, hasProxy := globalIPToPort[ipStr]
	if !hasProxy {
		port := 8080
		for {
			ln, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", port))
			if err == nil {
				globalIPToPort[ipStr] = port
				go func(listener net.Listener, targetIP string) {
					defer listener.Close()
					for {
						conn, err := listener.Accept()
						if err != nil {
							return
						}
						go handleProxy(conn, targetIP)
					}
				}(ln, ipStr)
				fmt.Printf("[VSta] TCP forward 127.0.0.1:%d -> %s:80\n", port, ipStr)
				break
			}
			port++
		}
	}
	port := globalIPToPort[ipStr]
	dhcpMutex.Unlock()

	udpFwdMu.Lock()
	uport := 5683
	if l, found := udpFwdListeners[ipStr]; found {
		uport = l.uport
	} else {
		for {
			uaddr, uerr := net.ResolveUDPAddr("udp", fmt.Sprintf("127.0.0.1:%d", uport))
			if uerr != nil {
				break
			}
			uconn, uerr := net.ListenUDP("udp", uaddr)
			if uerr == nil {
				udpFwdListeners[ipStr] = &udpFwdListener{uconn: uconn, uport: uport, room: room, mac: macString}
				go handleUDPProxy(uconn, ipStr)
				fmt.Printf("[VSta] UDP forward 127.0.0.1:%d/udp -> %s:5683\n", uport, ipStr)
				break
			}
			uport++
		}
	}
	udpFwdMu.Unlock()
	refreshUDPFwdRoom(ipStr, room, macString)

	client.WriteMutex.Lock()
	client.Conn.WriteMessage(websocket.TextMessage, []byte(fmt.Sprintf("PORT_FORWARD:http://127.0.0.1:%d", port)))
	client.Conn.WriteMessage(websocket.TextMessage, []byte(fmt.Sprintf("UDP_FORWARD:127.0.0.1:%d", uport)))
	client.WriteMutex.Unlock()
}

func vstaSendBoardText(client *Client, text string) {
	client.WriteMutex.Lock()
	client.Conn.WriteMessage(websocket.TextMessage, []byte(text))
	client.WriteMutex.Unlock()
}

// vstaDeliver is currently UNUSED (kept for a future second-board path).
// Delivering gateway 802.11 over the ROOM PIPE would break the room: the
// pipe is the shared hub<->gVisor Ethernet channel (QEMU protocol —
// length-prefixed). gVisor's AcceptQemu + the hub's gvisorToClientsLoop
// both consume it as ETHERNET, so raw 802.11 bytes would choke the stack
// and fan out as garbage. All live gateway-originated 802.11 goes via
// vstaDeliverBeacon (direct WS broadcast) instead.
func vstaDeliver(room *Room, frame []byte) {
	room.Lock()
	pipe := room.PipeToVN
	room.Unlock()
	if pipe == nil {
		return
	}
	vnPipeMu.Lock()
	defer vnPipeMu.Unlock()
	if werr := binary.Write(pipe, binary.BigEndian, uint32(len(frame))); werr != nil {
		return
	}
	pipe.Write(frame)
}

func vstaDeliverBeacon(room *Room, frame []byte) {
	// Gateway-originated 802.11 (beacon observer-copies, probe/auth/assoc
	// responses) goes via DIRECT WebSocket broadcast to ALL room peers
	// (raw MPDU, no EPWF mark — it originates here, not at the worker
	// tap), INCLUDING the requester.
	//
	// No sender exclusion: gorilla's concurrent writer (one WriteMutex
	// per client, hub + vsta paths both serialize on it) delivers
	// reliably to every peer — the earlier exclude-the-sender variant
	// silently dropped responses (requester never saw its own ASSOC-RESP,
	// and with only board+station in the room NOBODY did). Self-receipt
	// is harmless: the worker socket onmessage feeds non-ESP-NOW binary
	// frames into the MAC RX path when they parse as 802.11
	// ((bytes[0] & 0x0C) === 0x00), so the station's own uplink echo
	// re-enters MAC RX (ignored — wrong direction) while the AP-addressed
	// response is processed; py-vsta filters by fc0/subtype anyway.
	// The room pipe is NEVER touched (it is the shared hub<->gVisor
	// Ethernet channel; 802.11 bytes there would choke gVisor).
	room.Lock()
	targets := make([]*Client, 0, len(room.Clients))
	for c := range room.Clients {
		targets = append(targets, c)
	}
	room.Unlock()
	for _, c := range targets {
		c.WriteMutex.Lock()
		werr := c.Conn.WriteMessage(websocket.BinaryMessage, frame)
		c.WriteMutex.Unlock()
		if werr != nil {
			fmt.Printf("[VSta] deliver fc0=%#04x len=%d failed: %v\n", frame[0], len(frame), werr)
		}
	}
}

// ---- assoc tracking ------------------------------------------------------
// The board's esp_wifi_ap_get_sta_list() reads a native counter the engine
// owns; the gateway mirrors it so the page/Node harness can poll it.
// STACOUNT:<n> is a structured text message like BOARD_IP.

func vstaMarkDHCP(room *Room, staMAC net.HardwareAddr, ip net.IP) {
	vstaMu.Lock()
	vr := vstaRoomFor(room)
	key := staMAC.String()
	cl, ok := vr.clients[key]
	if !ok {
		cl = &vstaClient{mac: append(net.HardwareAddr(nil), staMAC...)}
		vr.clients[key] = cl
	}
	cl.ip = append(net.IP(nil), ip...)
	cl.dhcpDone = true
	vstaMu.Unlock()
}

func vstaSetAssoc(room *Room, staMAC net.HardwareAddr, assoc bool) {
	vstaMu.Lock()
	vr := vstaRoomFor(room)
	key := staMAC.String()
	cl, ok := vr.clients[key]
	if !ok {
		cl = &vstaClient{mac: append(net.HardwareAddr(nil), staMAC...)}
		vr.clients[key] = cl
	}
	cl.assoc = assoc
	n := 0
	for _, c := range vr.clients {
		if c.assoc {
			n++
		}
	}
	vstaMu.Unlock()
	room.Lock()
	targets := make([]*Client, 0, len(room.Clients))
	for c := range room.Clients {
		targets = append(targets, c)
	}
	room.Unlock()
	for _, c := range targets {
		vstaSendBoardText(c, fmt.Sprintf("STACOUNT:%d", n))
	}
}

func vstaAssocCount(room *Room) int {
	vstaMu.Lock()
	defer vstaMu.Unlock()
	vr, ok := vstaByRoom[room]
	if !ok || vr == nil {
		return 0
	}
	n := 0
	for _, c := range vr.clients {
		if c.assoc {
			n++
		}
	}
	return n
}

// ---- frame snoop entry point (called from the hub loop) ------------------
// Claims: probe req / auth / assoc req / disassoc / deauth to the AP under
// test, plus DHCP from virtual-station MACs. Returns true when consumed
// (caller skips VN pipe AND room broadcast).

func vstaIsApClientMAC(mac net.HardwareAddr) bool {
	// Virtual-station OUI 02:56:53 ("VST" with local bit).
	return len(mac) == 6 && mac[0] == 0x02 && mac[1] == 0x56 && mac[2] == 0x53
}

func vstaChecksum(data []byte) uint16 {
	sum := uint32(0)
	for i := 0; i+1 < len(data); i += 2 {
		sum += uint32(data[i])<<8 | uint32(data[i+1])
	}
	if len(data)%2 != 0 {
		sum += uint32(data[len(data)-1]) << 8
	}
	for sum>>16 != 0 {
		sum = (sum >> 16) + (sum & 0xFFFF)
	}
	return uint16(^sum)
}

func vstaHandleEcho(msg []byte, client *Client) bool {
	// Ethernet(14) + IPv4(20, no options) + ICMP(8+).
	if len(msg) < 14+20+8 {
		return false
	}
	if binary.BigEndian.Uint16(msg[12:14]) != 0x0800 || msg[14]>>4 != 4 {
		return false
	}
	ihl := int(msg[14]&0x0F) * 4
	if ihl != 20 || msg[14+9] != 1 { // ICMP only
		return false
	}
	srcMAC := net.HardwareAddr(append([]byte(nil), msg[6:12]...))
	if !vstaIsApClientMAC(srcMAC) {
		return false
	}
	ioff := 14 + ihl
	if msg[ioff] != 8 { // echo request only
		return false
	}
	srcIP := append(net.IP(nil), msg[14+12:14+16]...)
	dstIP := append(net.IP(nil), msg[14+16:14+20]...)
	// Only answer pings to the AP address; anything else is not ours
	// (and claiming it would blackhole real traffic).
	if !(dstIP[0] == 192 && dstIP[1] == 168 && dstIP[2] == 4 && dstIP[3] == 1) {
		return false
	}
	rep := make([]byte, len(msg))
	copy(rep, msg)
	// MAC swap: gateway MAC -> station.
	copy(rep[0:6], srcMAC)
	copy(rep[6:12], net.HardwareAddr{0x5a, 0x94, 0xef, 0xe4, 0x0c, 0xdd})
	// IP swap + rewrite header checksum.
	copy(rep[14+12:14+16], dstIP.To4())
	copy(rep[14+16:14+20], srcIP.To4())
	rep[14+10], rep[14+11] = 0, 0
	ck := vstaChecksum(rep[14 : 14+ihl])
	rep[14+10], rep[14+11] = byte(ck>>8), byte(ck)
	// ICMP type -> echo reply + rewrite checksum.
	rep[ioff] = 0
	rep[ioff+2], rep[ioff+3] = 0, 0
	ick := vstaChecksum(rep[ioff:])
	rep[ioff+2], rep[ioff+3] = byte(ick>>8), byte(ick)
	fmt.Printf("[VSta] ICMP echo %s -> reply\n", srcIP.String())
	client.WriteMutex.Lock()
	client.Conn.WriteMessage(websocket.BinaryMessage, rep)
	client.WriteMutex.Unlock()
	return true
}

func macEqual(a, b net.HardwareAddr) bool {
	if len(a) != 6 || len(b) != 6 {
		return false
	}
	for i := 0; i < 6; i++ {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// snoopVStaRaw is the hub-loop entry: parses just enough to route without
// letting 802.11 bytes hit the Ethernet (gopacket) path. EPWF-marked worker
// tap frames are stripped then handled; raw py-vsta mgmt frames handled
// directly; vsta-OUI DHCP handled via its own parse. Returns true when the
// caller must skip the VN pipe AND the room broadcast.
//
// NOTE: this is the ONLY vsta entry the hub calls (main.go). The older
// snoopVSta (packet-taking) wrapper below is kept for unit reuse; do not
// call it from the hub — it would double-parse and double-claim.
func snoopVStaRaw(msg []byte, client *Client, room *Room) bool {
	// EPWF-prefixed 802.11 medium frame from the worker tap: strip and route.
	if len(msg) > 4 && msg[0] == 0x45 && msg[1] == 0x50 && msg[2] == 0x57 && msg[3] == 0x46 {
		raw := msg[4:]
		if len(raw) >= 38 && (raw[0]&0x0C) == 0x00 && raw[0] == 0x80 {
			// Board beacon: learn the AP under test, then share the
			// stripped MPDU with room observers (the py-vsta proof
			// learns BSSID/SSID from it). Broadcast to ALL peers —
			// no sender exclusion (see vstaDeliverBeacon).
			// Claimed: beacons never touch the VN pipe or the Eth log.
			noteVStaBeacon(room, raw)
			vstaDeliverBeacon(room, raw)
			return true
		}
		if len(raw) >= 24 && (raw[0]&0x0C) == 0x00 {
			return snoopVStaDownlink(raw, client, room)
		}
		return true
	}
	// Raw (unmarked) 802.11 management frames from external stations.
	// Minimum management size is the 24-byte header (beacon check above
	// needs body, so it stays at 38). Classified by frame-control
	// type-0 + flags-0, NEVER by length alone: a 50-byte vsta echo/DHCP
	// frame can alias (bytes[0]&0x0C)==0 by accident (b0=0x02), but a
	// real management header always carries type 0 (low nibble 0) with
	// flags 0 (byte 1). The old bare length+low-bits check swallowed
	// every vsta ping/RS into snoopVStaMgmt, which dropped them as
	// "mgmt to unknown AP". Subtype is NOT constrained (assoc-req is
	// subtype 0, deauth is 12 — both must match); the residual alias
	// (ethernet dst C0:00:xx:...) is a multicast prefix that never
	// appears on this gateway.
	if len(msg) >= 24 && (msg[0]&0x0C) == 0x00 && msg[1] == 0x00 {
		if msg[0] == 0x80 {
			// A second board acting as prober teaches the AP the same
			// way the tap does; observers still get a copy, and raw
			// 802.11 must never reach gVisor as Ethernet.
			noteVStaBeacon(room, msg)
			vstaDeliverBeacon(room, msg)
			return true
		}
		if snoopVStaMgmt(msg, client, room) {
			return true
		}
		// Unhandled 802.11 (AP not learned yet, or frame not for this
		// AP): still claimed — raw 802.11 must never reach the VN pipe
		// (gVisor chokes) or misparse as Ethernet below.
		return true
	}
	// DHCP from a virtual-station MAC (Ethernet): answer as the AP's server.
	// Parse here (not via the caller's packet) so 802.11 never misparses.
	packet := gopacket.NewPacket(msg, layers.LayerTypeEthernet, gopacket.Default)
	return snoopVSta(msg, packet, client, room)
}

// snoopVStaDownlink handles AP-originated 802.11 management arriving via
// the worker EPWF tap (the board's own probe/auth/assoc responses, deauth,
// etc.). These are downlink frames for observers/capture — they must NEVER
// be answered (snoopVStaMgmt would mistake e.g. a broadcast deauth for a
// station request) and never fed to gVisor. Fanned to ALL room peers
// (no sender exclusion — see vstaDeliverBeacon), then claimed.
func snoopVStaDownlink(raw []byte, client *Client, room *Room) bool {
	vstaMu.Lock()
	vr, ok := vstaByRoom[room]
	var apMAC net.HardwareAddr
	if ok && vr != nil {
		apMAC = vr.apMAC
	}
	vstaMu.Unlock()
	if len(apMAC) == 0 {
		// Room not learned yet: swallow tap frames; there is nothing to
		// answer with until the first beacon teaches the AP identity.
		return true
	}
	if macEqual(net.HardwareAddr(raw[10:16]), apMAC) {
		vstaDeliverBeacon(room, raw)
		return true
	}
	// Not from the AP under test: run the station-uplink handler so a
	// first-speaker station still gets answered; unhandled frames stay
	// claimed (never gVisor, never Ethernet).
	if snoopVStaMgmt(raw, client, room) {
		return true
	}
	return true
}

func snoopVSta(msg []byte, packet gopacket.Packet, client *Client, room *Room) bool {
	// EPWF-prefixed 802.11 medium frames from the worker tap (beacons,
	// probe/auth/assoc responses, ACKs). Strip the magic, then route.
	// Beacons (subtype 8) are learned, not claimed.
	if len(msg) > 4 && msg[0] == 0x45 && msg[1] == 0x50 && msg[2] == 0x57 && msg[3] == 0x46 {
		raw := msg[4:]
		if len(raw) >= 38 && (raw[0]&0x0C) == 0x00 && raw[0] == 0x80 {
			noteVStaBeacon(room, raw)
			return true
		}
		if len(raw) >= 24 && (raw[0]&0x0C) == 0x00 {
			return snoopVStaMgmt(raw, client, room)
		}
		return true
	}
	// Raw (unmarked) 802.11 management frames: only from py-vsta style
	// external stations, which send them unmarked. Beacons learned here
	// too so a second board acting as prober still teaches the AP.
	// (Kept for unit reuse; the hub calls snoopVStaRaw, never this.)
	if len(msg) >= 24 && (msg[0]&0x0C) == 0x00 {
		if msg[0] == 0x80 && len(msg) >= 38 {
			noteVStaBeacon(room, msg)
			return false
		}
		return snoopVStaMgmt(msg, client, room)
	}
	// DHCP from a virtual-station MAC (Ethernet): answer as the AP's server.
	if udpLayer := packet.Layer(layers.LayerTypeUDP); udpLayer != nil {
		udp, _ := udpLayer.(*layers.UDP)
		if udp.DstPort == 67 {
			if ethLayer := packet.Layer(layers.LayerTypeEthernet); ethLayer != nil {
				eth, _ := ethLayer.(*layers.Ethernet)
				if vstaIsApClientMAC(eth.SrcMAC) {
					vstaAnswerDHCP(msg, packet, client, room)
					return true
				}
			}
		}
	}
	// ICMPv4 echo (ping) from a virtual-station MAC: answer directly as
	// 192.168.4.1. The station path has no v4-echo service (gVisor owns
	// 192.168.4.1 there and answers itself); AP clients live outside the
	// VN, so the gateway must answer or the proof's PING_OK never comes.
	if vstaHandleEcho(msg, client) {
		return true
	}
	// IPv6 RS from a virtual-station MAC: answer with a unicast RA via
	// the existing snoopIPv6 service (it keys on frame shape, not MAC).
	if snoopIPv6(msg, client, room) {
		return true
	}
	return false
}

// vstaHandleEcho answers an ICMPv4 echo request from a virtual-station
// MAC as 192.168.4.1 (the AP address the station pinged). Returns true
// when the frame was an echo request (claimed either way — a vsta-MAC
// frame must never fall through to gVisor as Ethernet).

func snoopVStaMgmt(msg []byte, client *Client, room *Room) bool {
	_ = client
	if len(msg) < 24 {
		return false
	}
	fc0 := msg[0]
	subtype := (fc0 >> 4) & 0x0F
	sa := net.HardwareAddr(append([]byte(nil), msg[10:16]...))
	vstaMu.Lock()
	vr, ok := vstaByRoom[room]
	if !ok || vr == nil {
		vstaMu.Unlock()
		return false
	}
	apMAC := append(net.HardwareAddr(nil), vr.apMAC...)
	vstaMu.Unlock()
	if len(apMAC) == 0 {
		// No AP learned yet on this room: still answer broadcast probes
		// with the gateway's own AP identity? No — without an AP under
		// test there is nothing to associate to. Log and claim (raw
		// 802.11 must never reach gVisor).
		fmt.Printf("[VSta] mgmt fc0=%#04x sub=%d sa=%s (no AP learned, drop)\n", fc0, subtype, sa.String())
		return true
	}
	// Only answer frames addressed to the AP under test (or broadcast
	// probes). Without this gate, one station's probe would elicit a
	// response storm addressed to every room in the gateway.
	da := net.HardwareAddr(append([]byte(nil), msg[4:10]...))
	isBcast := true
	for _, b := range da {
		if b != 0xFF {
			isBcast = false
			break
		}
	}
	if !isBcast && !macEqual(da, apMAC) {
		return false
	}
	fmt.Printf("[VSta] mgmt fc0=%#04x sub=%d sa=%s\n", fc0, subtype, sa.String())
	switch subtype {
	case 4: // probe req -> probe resp advertising the AP SSID
		vstaDeliverBeacon(room, vr.buildProbeResp(sa))
		return true
	case 11: // auth -> auth resp (seq 2)
		vstaDeliverBeacon(room, vr.buildAuthResp(sa))
		return true
	case 0, 2: // assoc req / reassoc req -> assoc resp + mark associated
		aid := uint16(1 + vstaAssocCount(room))
		vstaDeliverBeacon(room, vr.buildAssocResp(sa, aid))
		vstaSetAssoc(room, sa, true)
		return true
	case 10, 12: // disassoc / deauth -> mark gone
		vstaSetAssoc(room, sa, false)
		return true
	}
	return false
}

// vstaDeliverBeacon sends an 802.11 medium frame to the room's peers via
// direct WebSocket broadcast (raw MPDU, no EPWF mark — it originates here,
// not at the worker tap). The room pipe is NEVER touched (shared
// hub<->gVisor Ethernet channel; 802.11 bytes there would choke gVisor).
