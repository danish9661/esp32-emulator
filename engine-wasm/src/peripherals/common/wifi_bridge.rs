use crate::peripherals::types::*;

const WIFI_STATE_DISCONNECTED: u32 = 0;
const WIFI_STATE_CONNECTING: u32 = 1;
const WIFI_STATE_CONNECTED: u32 = 2;
const WIFI_STATE_ACCESS_POINT_NOT_FOUND: u32 = 3;
const WIFI_STATE_GATEWAY_ERROR: u32 = 4;

pub struct WifiStatus {
    pub state: u32,
    pub rx_frames: u32,
    pub rx_bytes: u32,
    pub tx_frames: u32,
    pub tx_bytes: u32,
    pub probe_request_count: u32,
    pub private_gateway: bool,
    pub error_message: [u8; 64],
    pub error_message_len: usize,
}

impl WifiStatus {
    pub fn new(private_gateway: bool) -> Self {
        WifiStatus {
            state: 0,
            rx_frames: 0,
            rx_bytes: 0,
            tx_frames: 0,
            tx_bytes: 0,
            probe_request_count: 0,
            private_gateway,
            error_message: [0u8; 64],
            error_message_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.state = 0;
        self.rx_frames = 0;
        self.rx_bytes = 0;
        self.tx_frames = 0;
        self.tx_bytes = 0;
        self.probe_request_count = 0;
        self.error_message = [0u8; 64];
        self.error_message_len = 0;
    }

    pub fn on_frame(&mut self, frame_type: u32, subtype: u32, length: u32) {
        if self.state == 0 {
            self.state = 1;
        }
        if frame_type == 0 && subtype == 4 {
            self.probe_request_count += 1;
        } else if frame_type == 0 && subtype == 5 {
            self.probe_request_count = 0;
        } else if frame_type == 2 {
            self.tx_frames += 1;
            self.tx_bytes += length;
        }
    }

    pub fn on_connected(&mut self) {
        self.state = 2;
        self.error_message = [0u8; 64];
        self.error_message_len = 0;
    }

    pub fn emit_stats(&mut self) {
    }
}

pub struct NativeInternetAP {
    pub ssid: [u8; 32],
    pub ssid_len: usize,
    pub password: [u8; 64],
    pub password_len: usize,
    pub channel: u32,
    pub seq: u32,
    pub bssid: [u8; 6],
    pub socket_active: bool,
    pub connected_clients: u32,
    pub status: WifiStatus,
    pub rx_listening: bool,
    pub packet_pending: bool,
    pub packet_data: [u8; 2048],
    pub packet_len: usize,
    pub ethernet_pending: bool,
    pub ethernet_data: [u8; 2048],
    pub ethernet_len: usize,
}

impl NativeInternetAP {
    pub fn new(
        ssid: &[u8],
        password: &[u8],
        channel: u32,
        private_gateway: bool,
        bssid_opt: Option<[u8; 6]>,
    ) -> Self {
        let mut ap = NativeInternetAP {
            ssid: [0u8; 32],
            ssid_len: 0,
            password: [0u8; 64],
            password_len: 0,
            channel,
            seq: 0,
            bssid: [0x42, 0x13, 0x37, 0x55, 0xaa, 0x01],
            socket_active: false,
            connected_clients: 0,
            status: WifiStatus::new(private_gateway),
            rx_listening: false,
            packet_pending: false,
            packet_data: [0u8; 2048],
            packet_len: 0,
            ethernet_pending: false,
            ethernet_data: [0u8; 2048],
            ethernet_len: 0,
        };
        let slen = core::cmp::min(ssid.len(), 32);
        ap.ssid[..slen].copy_from_slice(&ssid[..slen]);
        ap.ssid_len = slen;
        let plen = core::cmp::min(password.len(), 64);
        ap.password[..plen].copy_from_slice(&password[..plen]);
        ap.password_len = plen;
        if let Some(b) = bssid_opt {
            ap.bssid = b;
        }
        ap
    }

    pub fn next_seq(&mut self) -> u32 {
        self.seq = (self.seq + 1) % 4096;
        self.seq << 4
    }

    pub fn build_beacon(&mut self, buf: &mut [u8], time_us: u64) -> usize {
        buf[0] = 0x80;
        buf[1] = 0;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = 0xff;
        buf[5] = 0xff;
        buf[6] = 0xff;
        buf[7] = 0xff;
        buf[8] = 0xff;
        buf[9] = 0xff;
        buf[10] = self.bssid[0];
        buf[11] = self.bssid[1];
        buf[12] = self.bssid[2];
        buf[13] = self.bssid[3];
        buf[14] = self.bssid[4];
        buf[15] = self.bssid[5];
        buf[16] = self.bssid[0];
        buf[17] = self.bssid[1];
        buf[18] = self.bssid[2];
        buf[19] = self.bssid[3];
        buf[20] = self.bssid[4];
        buf[21] = self.bssid[5];
        let seq = self.next_seq();
        buf[22] = (seq & 0xff) as u8;
        buf[23] = ((seq >> 8) & 0xff) as u8;
        let mut t = time_us;
        let mut i = 0;
        while i < 8 {
            buf[24 + i] = (t & 0xff) as u8;
            t /= 256;
            i += 1;
        }
        buf[32] = 0x64;
        buf[33] = 0x00;
        buf[34] = 0x01;
        buf[35] = 0x04;
        buf[36] = 0;
        buf[37] = self.ssid_len as u8;
        let mut j = 0;
        while j < self.ssid_len {
            buf[38 + j] = self.ssid[j];
            j += 1;
        }
        let mut off = 38 + self.ssid_len;
        let rates: [u8; 10] = [1, 8, 0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24];
        let mut k = 0;
        while k < 10 {
            buf[off + k] = rates[k];
            k += 1;
        }
        off += 10;
        buf[off] = 3;
        buf[off + 1] = 1;
        buf[off + 2] = self.channel as u8;
        off += 3;
        buf[off] = 5;
        buf[off + 1] = 4;
        buf[off + 2] = 0;
        buf[off + 3] = 1;
        buf[off + 4] = 0;
        buf[off + 5] = 0;
        off += 6;
        off
    }

    pub fn build_ack(&self, da: &[u8; 6], buf: &mut [u8]) -> usize {
        buf[0] = 0xd4;
        buf[1] = 0x00;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = da[0];
        buf[5] = da[1];
        buf[6] = da[2];
        buf[7] = da[3];
        buf[8] = da[4];
        buf[9] = da[5];
        10
    }

    pub fn build_probe_response(
        &mut self,
        client_mac: &[u8; 6],
        requested_ssid: &[u8],
        req_ssid_len: usize,
        buf: &mut [u8],
    ) -> usize {
        buf[0] = 0x50;
        buf[1] = 0;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = client_mac[0];
        buf[5] = client_mac[1];
        buf[6] = client_mac[2];
        buf[7] = client_mac[3];
        buf[8] = client_mac[4];
        buf[9] = client_mac[5];
        buf[10] = self.bssid[0];
        buf[11] = self.bssid[1];
        buf[12] = self.bssid[2];
        buf[13] = self.bssid[3];
        buf[14] = self.bssid[4];
        buf[15] = self.bssid[5];
        buf[16] = self.bssid[0];
        buf[17] = self.bssid[1];
        buf[18] = self.bssid[2];
        buf[19] = self.bssid[3];
        buf[20] = self.bssid[4];
        buf[21] = self.bssid[5];
        let seq = self.next_seq();
        buf[22] = (seq & 0xff) as u8;
        buf[23] = ((seq >> 8) & 0xff) as u8;
        buf[24] = 0;
        buf[25] = 0;
        buf[26] = 0;
        buf[27] = 0;
        buf[28] = 0;
        buf[29] = 0;
        buf[30] = 0;
        buf[31] = 0;
        buf[32] = 0x64;
        buf[33] = 0x00;
        buf[34] = 0x01;
        buf[35] = 0x04;
        let (ssid_src, ssid_src_len) = if req_ssid_len > 0 {
            (requested_ssid, req_ssid_len)
        } else {
            (&self.ssid[..], self.ssid_len)
        };
        let adv_len = if ssid_src_len > 32 { 32 } else { ssid_src_len };
        buf[36] = 0;
        buf[37] = adv_len as u8;
        let mut i = 0;
        while i < adv_len {
            buf[38 + i] = ssid_src[i];
            i += 1;
        }
        let mut off = 38 + adv_len;
        let rates: [u8; 10] = [1, 8, 0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24];
        let mut k = 0;
        while k < 10 {
            buf[off + k] = rates[k];
            k += 1;
        }
        off += 10;
        buf[off] = 3;
        buf[off + 1] = 1;
        buf[off + 2] = self.channel as u8;
        off += 3;
        off
    }

    pub fn build_auth_response(&mut self, da: &[u8; 6], buf: &mut [u8]) -> usize {
        buf[0] = 0xb0;
        buf[1] = 0;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = da[0];
        buf[5] = da[1];
        buf[6] = da[2];
        buf[7] = da[3];
        buf[8] = da[4];
        buf[9] = da[5];
        buf[10] = self.bssid[0];
        buf[11] = self.bssid[1];
        buf[12] = self.bssid[2];
        buf[13] = self.bssid[3];
        buf[14] = self.bssid[4];
        buf[15] = self.bssid[5];
        buf[16] = self.bssid[0];
        buf[17] = self.bssid[1];
        buf[18] = self.bssid[2];
        buf[19] = self.bssid[3];
        buf[20] = self.bssid[4];
        buf[21] = self.bssid[5];
        let seq = self.next_seq();
        buf[22] = (seq & 0xff) as u8;
        buf[23] = ((seq >> 8) & 0xff) as u8;
        buf[24] = 0;
        buf[25] = 0;
        buf[26] = 2;
        buf[27] = 0;
        buf[28] = 0;
        buf[29] = 0;
        30
    }

    pub fn build_assoc_response(&mut self, da: &[u8; 6], buf: &mut [u8]) -> usize {
        buf[0] = 0x10;
        buf[1] = 0;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = da[0];
        buf[5] = da[1];
        buf[6] = da[2];
        buf[7] = da[3];
        buf[8] = da[4];
        buf[9] = da[5];
        buf[10] = self.bssid[0];
        buf[11] = self.bssid[1];
        buf[12] = self.bssid[2];
        buf[13] = self.bssid[3];
        buf[14] = self.bssid[4];
        buf[15] = self.bssid[5];
        buf[16] = self.bssid[0];
        buf[17] = self.bssid[1];
        buf[18] = self.bssid[2];
        buf[19] = self.bssid[3];
        buf[20] = self.bssid[4];
        buf[21] = self.bssid[5];
        let seq = self.next_seq();
        buf[22] = (seq & 0xff) as u8;
        buf[23] = ((seq >> 8) & 0xff) as u8;
        buf[24] = 0x11;
        buf[25] = 0x04;
        buf[26] = 0;
        buf[27] = 0;
        buf[28] = 1;
        buf[29] = 192;
        let rates: [u8; 10] = [1, 8, 0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24];
        let mut k = 0;
        while k < 10 {
            buf[30 + k] = rates[k];
            k += 1;
        }
        40
    }

    pub fn on_ethernet_rx(&mut self, eth: &[u8], buf: &mut [u8]) -> usize {
        if eth.len() < 14 || !self.rx_listening {
            return 0;
        }
        let wifi_len = 32 + eth.len() - 14;
        buf[0] = 0x08;
        buf[1] = 0x02;
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = eth[0];
        buf[5] = eth[1];
        buf[6] = eth[2];
        buf[7] = eth[3];
        buf[8] = eth[4];
        buf[9] = eth[5];
        buf[10] = self.bssid[0];
        buf[11] = self.bssid[1];
        buf[12] = self.bssid[2];
        buf[13] = self.bssid[3];
        buf[14] = self.bssid[4];
        buf[15] = self.bssid[5];
        buf[16] = eth[6];
        buf[17] = eth[7];
        buf[18] = eth[8];
        buf[19] = eth[9];
        buf[20] = eth[10];
        buf[21] = eth[11];
        let seq = self.next_seq();
        buf[22] = (seq & 0xff) as u8;
        buf[23] = ((seq >> 8) & 0xff) as u8;
        buf[24] = 0xaa;
        buf[25] = 0xaa;
        buf[26] = 0x03;
        buf[27] = 0;
        buf[28] = 0;
        buf[29] = 0;
        buf[30] = eth[12];
        buf[31] = eth[13];
        let mut i = 0;
        while i < eth.len() - 14 {
            buf[32 + i] = eth[14 + i];
            i += 1;
        }
        wifi_len
    }

    pub fn send_ethernet(&mut self, eth: &[u8]) {
        let len = if eth.len() > 2048 { 2048 } else { eth.len() };
        let mut i = 0;
        while i < len {
            self.ethernet_data[i] = eth[i];
            i += 1;
        }
        self.ethernet_len = len;
        self.ethernet_pending = true;
        // Deliver synchronously (JS parity: sendEthernet pushes pcap + sends
        // the socket immediately). The worker reads the frame back from the
        // static while this call is on the stack.
        unsafe {
            crate::peripherals::common::ffi::js_wifi_ap_send_eth(
                self.ethernet_data.as_ptr() as u32,
                len as u32,
            );
        }
    }

    pub fn handle_wifi_frame(&mut self, frame: &[u8], channel: Option<u32>) {
        if let Some(ch) = channel {
            if ch != self.channel {
                return;
            }
        }
        if frame.len() < 24 {
            return;
        }
        let fc1 = frame[0];
        let frame_type = ((fc1 as u32) >> 2) & 3;
        let subtype = ((fc1 as u32) >> 4) & 15;
        let da: [u8; 6] = [
            frame[4], frame[5], frame[6], frame[7], frame[8], frame[9],
        ];
        let sa: [u8; 6] = [
            frame[10], frame[11], frame[12], frame[13], frame[14], frame[15],
        ];
        self.status.on_frame(frame_type, subtype, frame.len() as u32);
        let mut is_broadcast = true;
        let mut bi = 0;
        while bi < 6 {
            if da[bi] != 0xff {
                is_broadcast = false;
                break;
            }
            bi += 1;
        }
        if !is_broadcast && self.rx_listening {
            let mut ack_buf = [0u8; 10];
            let ack_len = self.build_ack(&sa, &mut ack_buf);
            self.push_output(&ack_buf[..ack_len]);
        }
        if frame_type == 0 {
            if subtype == 4 {
                let mut req_ssid_buf = [0u8; 32];
                let mut req_ssid_len: usize = 0;
                if frame.len() > 26 {
                    let mut pos: usize = 24;
                    while pos + 2 <= frame.len() {
                        if frame[pos] == 0 && frame[pos + 1] > 0 {
                            let slen = frame[pos + 1] as usize;
                            let copy_len = if slen > 32 { 32 } else { slen };
                            let mut si = 0;
                            while si < copy_len {
                                req_ssid_buf[si] = frame[pos + 2 + si];
                                si += 1;
                            }
                            req_ssid_len = copy_len;
                            break;
                        }
                        pos += 2 + frame[pos + 1] as usize;
                    }
                }
                if self.rx_listening {
                    let mut pr_buf = [0u8; 256];
                    let pr_len = self.build_probe_response(
                        &sa,
                        &req_ssid_buf[..req_ssid_len],
                        req_ssid_len,
                        &mut pr_buf,
                    );
                    self.push_output(&pr_buf[..pr_len]);
                }
            } else if subtype == 11 {
                if self.rx_listening {
                    let mut auth_buf = [0u8; 30];
                    let auth_len = self.build_auth_response(&sa, &mut auth_buf);
                    self.push_output(&auth_buf[..auth_len]);
                }
            } else if subtype == 0 {
                self.connected_clients += 1;
                self.connect_gateway();
                if self.rx_listening {
                    let mut assoc_buf = [0u8; 40];
                    let assoc_len = self.build_assoc_response(&sa, &mut assoc_buf);
                    self.push_output(&assoc_buf[..assoc_len]);
                }
            }
        } else if frame_type == 2 {
            let is_qos = subtype == 8;
            let mh_len: usize = if is_qos { 26 } else { 24 };
            if frame.len() > mh_len + 8
                && frame[mh_len] == 0xaa
                && frame[mh_len + 1] == 0xaa
            {
                let pay_len = frame.len() - (mh_len + 8);
                let eth_len = if 60 > 14 + pay_len { 60 } else { 14 + pay_len };
                let mut eth = [0u8; 2048];
                eth[0] = frame[16];
                eth[1] = frame[17];
                eth[2] = frame[18];
                eth[3] = frame[19];
                eth[4] = frame[20];
                eth[5] = frame[21];
                eth[6] = frame[10];
                eth[7] = frame[11];
                eth[8] = frame[12];
                eth[9] = frame[13];
                eth[10] = frame[14];
                eth[11] = frame[15];
                eth[12] = frame[mh_len + 6];
                eth[13] = frame[mh_len + 7];
                let mut pi = 0;
                while pi < pay_len {
                    eth[14 + pi] = frame[mh_len + 8 + pi];
                    pi += 1;
                }
                if eth[12] == 0x08 && eth[13] == 0x06 && eth_len >= 42 {
                    eth[22] = frame[10];
                    eth[23] = frame[11];
                    eth[24] = frame[12];
                    eth[25] = frame[13];
                    eth[26] = frame[14];
                    eth[27] = frame[15];
                }
                self.send_ethernet(&eth[..eth_len]);
            }
        }
    }

    fn push_output(&mut self, data: &[u8]) {
        let len = if data.len() > 2048 { 2048 } else { data.len() };
        let mut i = 0;
        while i < len {
            self.packet_data[i] = data[i];
            i += 1;
        }
        self.packet_len = len;
        self.packet_pending = true;
        // JS parity: onRxCb({ data }) fired synchronously for every response
        // frame (ACK, probe/auth/assoc responses, beacon, eth→wifi wrap).
        unsafe {
            crate::peripherals::common::ffi::js_wifi_ap_rx_frame(
                self.packet_data.as_ptr() as u32,
                len as u32,
            );
        }
    }

    pub fn set_rx_cb(&mut self) {
        self.rx_listening = true;
    }

    pub fn connect_gateway(&mut self) {
        if self.socket_active {
            return;
        }
        self.socket_active = true;
        // JS parity: connectGateway() opens the worker-side WebSocket (called
        // on assoc and once at setup — the worker ignores duplicate calls).
        unsafe {
            crate::peripherals::common::ffi::js_wifi_ap_connected();
        }
    }

    pub fn process_beacon(&mut self, time_us: u64) {
        if !self.rx_listening {
            return;
        }
        let mut buf = [0u8; 256];
        let len = self.build_beacon(&mut buf, time_us);
        self.push_output(&buf[..len]);
    }

    pub fn receive_from_host(&mut self, data: &[u8]) {
        self.status.rx_frames += 1;
        self.status.rx_bytes += data.len() as u32;
        self.status.on_connected();
        self.status.emit_stats();
        let mut wifi_buf = [0u8; 2048];
        let wifi_len = self.on_ethernet_rx(data, &mut wifi_buf);
        if wifi_len > 0 {
            self.push_output(&wifi_buf[..wifi_len]);
        }
    }

    pub fn get_pcap_data(&self, buf: &mut [u8]) -> usize {
        if buf.len() < 24 {
            return 0;
        }
        buf[0] = 0xa1;
        buf[1] = 0xb2;
        buf[2] = 0xc3;
        buf[3] = 0xd4;
        buf[4] = 0;
        buf[5] = 2;
        buf[6] = 0;
        buf[7] = 4;
        buf[8] = 0;
        buf[9] = 0;
        buf[10] = 0;
        buf[11] = 0;
        buf[12] = 0;
        buf[13] = 0;
        buf[14] = 0;
        buf[15] = 0;
        buf[16] = 0;
        buf[17] = 0;
        buf[18] = 0xff;
        buf[19] = 0xff;
        buf[20] = 0;
        buf[21] = 0;
        buf[22] = 0;
        buf[23] = 1;
        24
    }

    pub fn clear_pcap_buffer(&mut self) {
    }

    pub fn disconnect(&mut self) {
        self.socket_active = false;
    }
}

pub struct NativeWifiMedium;

impl NativeWifiMedium {
    pub fn new() -> Self {
        NativeWifiMedium
    }

    pub fn transmit(ap: &mut NativeInternetAP, frame: &[u8], channel: Option<u32>) {
        ap.handle_wifi_frame(frame, channel);
    }

    pub fn listen(ap: &mut NativeInternetAP) {
        ap.set_rx_cb();
    }
}

pub struct NativeWiFiBridge {
    pub ap: NativeInternetAP,
    pub medium: NativeWifiMedium,
}

impl NativeWiFiBridge {
    pub fn new(
        ssid: &[u8],
        password: &[u8],
        channel: u32,
        private_gateway: bool,
        bssid: Option<[u8; 6]>,
    ) -> Self {
        NativeWiFiBridge {
            ap: NativeInternetAP::new(ssid, password, channel, private_gateway, bssid),
            medium: NativeWifiMedium::new(),
        }
    }

    pub fn disconnect(&mut self) {
        self.ap.disconnect();
    }
}

// ---- Native WiFi AP wiring ----
// NativeInternetAP port wired as an active module (JS parity with the
// worker's NativeWiFiBridge). The AP owns the 802.11 state machine and
// frame builders; the worker owns the gateway WebSocket, pcap records and
// RX delivery (sendFrame). All frame/status data crosses the boundary
// through the AP statics below (js_wifi_ap_rx_frame/send_eth/connected).
// Storage: one ~1.2KB struct + one 2KB scratch — SAFE since the JS memory
// regions moved to 0x400000 (AGENTS.md 4d40024 lesson: statics must never
// land on live PTE entries — re-verify with tests/diag-maphist.mjs).

static mut WIFI_AP_INIT: bool = false;
static mut WIFI_AP: Option<NativeInternetAP> = None;
static mut WIFI_AP_SCRATCH: [u8; 2048] = [0; 2048];

fn wifi_ap() -> &'static mut NativeInternetAP {
    unsafe { WIFI_AP.as_mut().expect("WiFi AP not initialized") }
}

fn read_mem_u8(addr: u32, off: usize) -> u8 {
    unsafe { core::ptr::read_volatile((addr as usize + off) as *const u8) }
}

#[no_mangle]
pub extern "C" fn native_wifi_ap_init(
    ssid_ptr: u32,
    ssid_len: u32,
    bssid_ptr: u32,
    bssid_len: u32,
    channel: u32,
    private_gateway: u32,
) {
    unsafe {
        if WIFI_AP_INIT {
            return;
        }
        let slen = core::cmp::min(ssid_len as usize, 32);
        let mut ssid = [0u8; 32];
        let mut i = 0;
        while i < slen {
            ssid[i] = read_mem_u8(ssid_ptr, i);
            i += 1;
        }
        let bssid_opt = if bssid_len == 6 {
            Some([
                read_mem_u8(bssid_ptr, 0),
                read_mem_u8(bssid_ptr, 1),
                read_mem_u8(bssid_ptr, 2),
                read_mem_u8(bssid_ptr, 3),
                read_mem_u8(bssid_ptr, 4),
                read_mem_u8(bssid_ptr, 5),
            ])
        } else {
            None
        };
        let mut ap = NativeInternetAP::new(
            &ssid[..slen],
            &[],
            channel,
            private_gateway != 0,
            bssid_opt,
        );
        // JS parity: medium.listen(cb) — responses/beacons delivered via FFI.
        ap.set_rx_cb();
        WIFI_AP = Some(ap);
        WIFI_AP_INIT = true;
    }
}

// Worker-side frame input buffer (TX frames + eth RX frames are written
// here before calling handle_tx / eth_rx).
#[no_mangle]
pub extern "C" fn native_wifi_ap_scratch() -> u32 {
    unsafe { &mut WIFI_AP_SCRATCH as *mut [u8; 2048] as u32 }
}

// Board TX frame (written to the scratch) — JS parity with
// medium.transmit({ data: frame, channel }) → handleWifiFrame.
#[no_mangle]
pub extern "C" fn native_wifi_ap_handle_tx(frame_len: u32, channel: u32) {
    unsafe {
        if !WIFI_AP_INIT {
            return;
        }
        let len = core::cmp::min(frame_len as usize, WIFI_AP_SCRATCH.len());
        if len < 1 {
            return;
        }
        let scratch = &mut *(&mut WIFI_AP_SCRATCH as *mut [u8; 2048]);
        let ch = if channel != 0 { Some(channel) } else { None };
        wifi_ap().handle_wifi_frame(&scratch[..len], ch);
    }
}

// Ethernet frame from the gateway (written to the scratch) — JS parity with
// socket.onmessage → status counters + onEthernetRx.
#[no_mangle]
pub extern "C" fn native_wifi_ap_eth_rx(eth_len: u32) {
    unsafe {
        if !WIFI_AP_INIT {
            return;
        }
        let len = core::cmp::min(eth_len as usize, WIFI_AP_SCRATCH.len());
        if len < 1 {
            return;
        }
        let scratch = &mut *(&mut WIFI_AP_SCRATCH as *mut [u8; 2048]);
        wifi_ap().receive_from_host(&scratch[..len]);
    }
}

// Beacon delivery — JS parity with the clock event (clock.createEvent at
// 102ms sim-time) calling buildBeacon → onRxCb.
#[no_mangle]
pub extern "C" fn native_wifi_ap_send_beacon() {
    unsafe {
        if !WIFI_AP_INIT {
            return;
        }
        let now_us = (crate::native_mmio::clk_nanos() as f64 / 1000.0) as u64;
        wifi_ap().process_beacon(now_us);
    }
}

// Status snapshot for the worker's SAB slots / CMD_GET_WIFI_STATS: 7 × i32 LE
// (state, txFrames, txBytes, rxFrames, rxBytes, probeRequestCount,
// connectedClients).
#[no_mangle]
pub extern "C" fn native_wifi_ap_get_status(ptr: u32) {
    unsafe {
        if !WIFI_AP_INIT {
            return;
        }
        let ap = wifi_ap();
        let p = ptr as *mut i32;
        core::ptr::write_unaligned(p.add(0), ap.status.state as i32);
        core::ptr::write_unaligned(p.add(1), ap.status.tx_frames as i32);
        core::ptr::write_unaligned(p.add(2), ap.status.tx_bytes as i32);
        core::ptr::write_unaligned(p.add(3), ap.status.rx_frames as i32);
        core::ptr::write_unaligned(p.add(4), ap.status.rx_bytes as i32);
        core::ptr::write_unaligned(p.add(5), ap.status.probe_request_count as i32);
        core::ptr::write_unaligned(p.add(6), ap.connected_clients as i32);
    }
}
