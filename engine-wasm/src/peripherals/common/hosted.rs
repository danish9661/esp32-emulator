use crate::peripherals::types::*;

const BROADCAST_MAC: [u8; 6] = [255; 6];
const HCI_PACKET_HEADER_SIZE: u32 = 12;
const SERIAL_TAG_ENDPOINT: u8 = 1;
const SERIAL_TAG_DATA: u8 = 2;
const RPC_RESPONSE_ENDPOINT: [u8; 6] = [82, 80, 67, 82, 115, 112];
const RPC_EVENT_ENDPOINT: [u8; 6] = [82, 80, 67, 69, 118, 116];
const SDIO_INT_STATUS_REG: u32 = 80;
const SDIO_INT_ENABLE_REG: u32 = 88;
const SDIO_TX_BYTE_COUNT_REG: u32 = 96;
const SDIO_TX_BUF_COUNT_REG: u32 = 68;
const SDIO_INT_CLEAR_REG: u32 = 212;
const SDIO_INT_ENABLE_SET_REG: u32 = 220;
const SDIO_DATA_PATH_CTRL_REG: u32 = 140;
const TX_INT_MASK: u32 = 8388608;
const DEFAULT_BLOCK_SIZE: u32 = 512;
const CMD_GO_IDLE_STATE: u32 = 0;
const CMD_IO_SEND_OP_COND: u32 = 5;
const CMD_SEND_RELATIVE_ADDR: u32 = 3;
const CMD_SELECT_CARD: u32 = 7;
const CMD_IO_RW_DIRECT: u32 = 52;
const CMD_IO_RW_EXTENDED: u32 = 53;
const HOSTED_EVENT_REARM_INTERRUPT: u32 = 40;
const HOSTED_EVENT_STA_START: u32 = 41;
const HOSTED_EVENT_STA_CONNECTED: u32 = 42;
const HOSTED_EVENT_SCAN_COMPLETE: u32 = 43;
const MAX_PACKET_SIZE: usize = 2048;
const MAX_QUEUED_PACKETS: usize = 64;
const MAX_SCAN_RESULTS: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WifiElementId {
    SSID = 0,
    SupportedRates = 1,
    FHParameterSet = 2,
    DSParameterSet = 3,
    CFParameterSet = 4,
    TrafficIndicationMap = 5,
    IBSSParameterSet = 6,
    Country = 7,
    HoppingPatternParameters = 8,
    HoppingPatternTable = 9,
    Request = 10,
    ChallengeText = 16,
    PowerConstraint = 32,
    PowerCapability = 33,
    TPCRequest = 34,
    TPCReport = 35,
    SupportedChannels = 36,
    ChannelSwitchAnnouncement = 37,
    MeasurementRequest = 38,
    MeasurementReport = 39,
    Quiet = 40,
    IBSSDFS = 41,
    ERPInformation = 42,
    RobustSecurityNetwork = 48,
    ExtendedSupportedRates = 50,
    WPA = 221,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Management = 0,
    Control = 1,
    Data = 2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameSubtype {
    AssociationRequest = 0,
    ReassociationRequest = 2,
    ProbeRequest = 4,
    TimingAdvertisement = 6,
    Beacon = 8,
    Disassociation = 10,
    Deauthentication = 12,
    Authentication = 11,
    Action = 14,
    AssociationResponse = 1,
    ReassociationResponse = 3,
    ProbeResponse = 5,
    Reserved = 7,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SdioCardState {
    Idle = 0,
    Ready = 1,
    Standby = 2,
    Command = 3,
    Transfer = 4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WifiStationState {
    Disconnected = 0,
    Scanning = 1,
    Authenticating = 2,
    Associating = 3,
    Connected = 4,
}

#[derive(Clone, Copy, Default)]
pub struct HciHeader {
    pub if_type: u32,
    pub if_num: u32,
    pub flags: u32,
    pub len: u32,
    pub offset: u32,
    pub checksum: u32,
    pub seq_num: u32,
    pub throttle_cmd: u32,
    pub hci_pkt_type: u32,
}

#[derive(Clone, Copy, Default)]
pub struct ProtobufMsg {
    pub msg_type: u32,
    pub msg_id: u32,
    pub uid: u32,
    pub payload_offset: u32,
    pub payload_field_num: u32,
}

#[derive(Clone, Copy)]
pub struct ScanResult {
    pub bssid: [u8; 6],
    pub ssid: [u8; 32],
    pub ssid_len: u32,
    pub channel: u32,
    pub rssi: i32,
    pub authmode: u32,
}

pub fn build_wifi_element(id: u8, data: &[u8], buf: &mut [u8]) -> usize {
    let n = data.len().min(32);
    buf[0] = id;
    buf[1] = n as u8;
    buf[2..2 + n].copy_from_slice(&data[..n]);
    2 + n
}

pub fn extract_arg_ssid<'a>(ssid: &'a [u8]) -> &'a [u8] {
    ssid
}

fn decode_text(bytes: &[u8], buf: &mut [u8]) -> usize {
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    n
}

fn read_u16_le(data: &[u8], offset: usize) -> u32 {
    (data[offset + 1] as u32) << 8 | data[offset] as u32
}

pub fn parse_frame_ctrl(frame_hdr: &[u8]) -> (u32, u32, u32, u32, u32) {
    let version = 3 & frame_hdr[0] as u32;
    let type_val = (frame_hdr[0] as u32 >> 2) & 3;
    let subtype = (frame_hdr[0] as u32 >> 4) & 15;
    let flags = frame_hdr[1] as u32;
    let duration = ((frame_hdr[3] as u32) >> 8) + frame_hdr[2] as u32;
    (version, type_val, subtype, flags, duration)
}

fn build_frame_ctrl(frame_type: u32, frame_subtype: u32) -> u8 {
    (((3 & frame_type) << 2) | ((15 & frame_subtype) << 4)) as u8
}

fn parse_wifi_elements(payload: &[u8], results: &mut [(u8, u32); 32]) -> u32 {
    let mut pos = 0;
    let mut count = 0;
    while pos + 1 < payload.len() {
        let elem_id = payload[pos];
        let elem_len = payload[pos + 1] as usize;
        pos += 2;
        if payload.len() >= pos + elem_len {
            if count < 32 {
                results[count as usize] = (elem_id, pos as u32);
                count += 1;
            }
            pos += elem_len;
        }
    }
    count
}

fn parse_beacon_frame(frame_data: &[u8], ssid_out: &mut [u8], ssid_len_out: &mut u32, bssid_out: &mut [u8; 6], channel_out: &mut u32) {
    if frame_data.len() >= 22 {
        bssid_out.copy_from_slice(&frame_data[16..22]);
    }
    let mut elements = [(0u8, 0u32); 32];
    let count = if frame_data.len() > 36 { parse_wifi_elements(&frame_data[36..], &mut elements) } else { 0 };
    let mut i = 0;
    while (i as u32) < count {
        let (id, off) = elements[i as usize];
        if id == WifiElementId::SSID as u8 {
            let start = off as usize;
            let elem_hdr = &frame_data[36 + start - 2..];
            let len = elem_hdr[1] as usize;
            let n = decode_text(&frame_data[36 + start..36 + start + len], ssid_out);
            *ssid_len_out = n as u32;
        }
        if id == WifiElementId::DSParameterSet as u8 {
            let start = off as usize;
            if frame_data.len() > 36 + start {
                *channel_out = frame_data[36 + start] as u32;
            }
        }
        i += 1;
    }
}

fn parse_probe_resp_frame(frame_data: &[u8], ssid_out: &mut [u8], ssid_len_out: &mut u32, bssid_out: &mut [u8; 6], channel_out: &mut u32) {
    parse_beacon_frame(frame_data, ssid_out, ssid_len_out, bssid_out, channel_out);
}

fn parse_auth_frame(data: &[u8], bssid_out: &mut [u8; 6], algorithm_out: &mut u32, auth_seq_out: &mut u32, status_out: &mut u32) {
    if data.len() >= 22 {
        bssid_out.copy_from_slice(&data[16..22]);
    }
    *algorithm_out = if data.len() >= 26 { read_u16_le(data, 24) } else { 0 };
    *auth_seq_out = if data.len() >= 28 { read_u16_le(data, 26) } else { 0 };
    *status_out = if data.len() >= 30 { read_u16_le(data, 28) } else { 0 };
}

fn parse_assoc_resp_frame(data: &[u8], bssid_out: &mut [u8; 6], capabilities_out: &mut u32, status_out: &mut u32, association_id_out: &mut u32) {
    if data.len() >= 22 {
        bssid_out.copy_from_slice(&data[16..22]);
    }
    *capabilities_out = if data.len() >= 26 { read_u16_le(data, 24) } else { 0 };
    *status_out = if data.len() >= 28 { read_u16_le(data, 26) } else { 0 };
    *association_id_out = if data.len() >= 30 { 16383 & read_u16_le(data, 28) } else { 0 };
}

fn build_probe_req_frame(src: &[u8; 6], ssid: &[u8], seq: u32, buf: &mut [u8]) -> usize {
    let mut pos = 0;
    buf[pos] = build_frame_ctrl(FrameType::Management as u32, FrameSubtype::ProbeRequest as u32);
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    for &b in &BROADCAST_MAC { buf[pos] = b; pos += 1; }
    for &b in src { buf[pos] = b; pos += 1; }
    for &b in &BROADCAST_MAC { buf[pos] = b; pos += 1; }
    buf[pos] = ((seq >> 4) & 255) as u8;
    pos += 1;
    buf[pos] = ((seq << 4) & 255) as u8;
    pos += 1;
    let mut elem_buf = [0u8; 34];
    let n = build_wifi_element(WifiElementId::SSID as u8, ssid, &mut elem_buf);
    for &b in &elem_buf[..n] { buf[pos] = b; pos += 1; }
    let rates = [140u8, 18, 152, 36, 176, 72, 96, 108];
    let n2 = build_wifi_element(WifiElementId::SupportedRates as u8, &rates, &mut elem_buf);
    for &b in &elem_buf[..n2] { buf[pos] = b; pos += 1; }
    pos
}

fn build_auth_frame(src: &[u8; 6], bssid: &[u8; 6], seq: u32, buf: &mut [u8]) -> usize {
    let mut pos = 0;
    buf[pos] = build_frame_ctrl(FrameType::Management as u32, FrameSubtype::Authentication as u32);
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    for &b in bssid { buf[pos] = b; pos += 1; }
    for &b in src { buf[pos] = b; pos += 1; }
    for &b in bssid { buf[pos] = b; pos += 1; }
    buf[pos] = ((seq >> 4) & 255) as u8;
    pos += 1;
    buf[pos] = ((seq << 4) & 255) as u8;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 1;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    pos
}

fn build_assoc_req_frame(src: &[u8; 6], bssid: &[u8; 6], ssid: &[u8], seq: u32, buf: &mut [u8]) -> usize {
    let mut pos = 0;
    buf[pos] = build_frame_ctrl(FrameType::Management as u32, FrameSubtype::AssociationRequest as u32);
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    for &b in bssid { buf[pos] = b; pos += 1; }
    for &b in src { buf[pos] = b; pos += 1; }
    for &b in bssid { buf[pos] = b; pos += 1; }
    buf[pos] = ((seq >> 4) & 255) as u8;
    pos += 1;
    buf[pos] = ((seq << 4) & 255) as u8;
    pos += 1;
    buf[pos] = 33;
    pos += 1;
    buf[pos] = 4;
    pos += 1;
    buf[pos] = 10;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    let mut elem_buf = [0u8; 34];
    let n = build_wifi_element(WifiElementId::SSID as u8, ssid, &mut elem_buf);
    for &b in &elem_buf[..n] { buf[pos] = b; pos += 1; }
    let rates = [140u8, 18, 152, 36, 176, 72, 96, 108];
    let n2 = build_wifi_element(WifiElementId::SupportedRates as u8, &rates, &mut elem_buf);
    for &b in &elem_buf[..n2] { buf[pos] = b; pos += 1; }
    pos
}

fn extract_data_payload(frame_data: &[u8], buf: &mut [u8]) -> usize {
    let has_addr4 = 128 & frame_data[0] as u32;
    let from_ds = 1 & frame_data[1] as u32;
    let to_ds = 2 & frame_data[1] as u32;
    let mut pos = 0;
    if from_ds != 0 && to_ds == 0 {
        for &b in &frame_data[16..22] { buf[pos] = b; pos += 1; }
        for &b in &frame_data[10..16] { buf[pos] = b; pos += 1; }
    } else if from_ds == 0 && to_ds != 0 {
        for &b in &frame_data[4..10] { buf[pos] = b; pos += 1; }
        for &b in &frame_data[16..22] { buf[pos] = b; pos += 1; }
    } else {
        for &b in &frame_data[16..22] { buf[pos] = b; pos += 1; }
        for &b in &frame_data[10..16] { buf[pos] = b; pos += 1; }
    }
    let payload_start = if has_addr4 != 0 { 32 } else { 30 };
    for &b in &frame_data[payload_start..] { buf[pos] = b; pos += 1; }
    pos
}

fn build_data_frame(frame_data: &[u8], bssid: &[u8; 6], buf: &mut [u8]) -> usize {
    let mut pos = 0;
    buf[pos] = build_frame_ctrl(FrameType::Data as u32, 0);
    pos += 1;
    buf[pos] = 1;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    for &b in bssid { buf[pos] = b; pos += 1; }
    for &b in &frame_data[6..12] { buf[pos] = b; pos += 1; }
    for &b in &frame_data[0..6] { buf[pos] = b; pos += 1; }
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 170;
    pos += 1;
    buf[pos] = 170;
    pos += 1;
    buf[pos] = 3;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    buf[pos] = 0;
    pos += 1;
    for &b in &frame_data[12..] { buf[pos] = b; pos += 1; }
    pos
}

pub fn parse_hci_header(buf: &[u8]) -> HciHeader {
    let idx_val = buf[0] as u32;
    HciHeader {
        if_type: 15 & idx_val,
        if_num: (idx_val >> 4) & 15,
        flags: buf[1] as u32,
        len: (buf[2] as u32) | ((buf[3] as u32) << 8),
        offset: (buf[4] as u32) | ((buf[5] as u32) << 8),
        checksum: (buf[6] as u32) | ((buf[7] as u32) << 8),
        seq_num: (buf[8] as u32) | ((buf[9] as u32) << 8),
        throttle_cmd: 3 & buf[10] as u32,
        hci_pkt_type: buf[11] as u32,
    }
}

pub fn serialize_hci_header(hdr: &HciHeader, buf: &mut [u8]) {
    buf[0] = ((15 & hdr.if_type) | ((15 & hdr.if_num) << 4)) as u8;
    buf[1] = hdr.flags as u8;
    buf[2] = hdr.len as u8;
    buf[3] = (hdr.len >> 8) as u8;
    buf[4] = hdr.offset as u8;
    buf[5] = (hdr.offset >> 8) as u8;
    buf[6] = hdr.checksum as u8;
    buf[7] = (hdr.checksum >> 8) as u8;
    buf[8] = hdr.seq_num as u8;
    buf[9] = (hdr.seq_num >> 8) as u8;
    buf[10] = (3 & hdr.throttle_cmd) as u8;
    buf[11] = hdr.hci_pkt_type as u8;
}

fn compute_checksum16(data: &[u8], len: u32) -> u32 {
    let mut sum = 0u32;
    let n = (len as usize).min(data.len());
    for i in 0..n {
        sum += data[i] as u32;
    }
    65535 & sum
}

fn decode_varint(data: &[u8], offset: usize) -> (u32, u32) {
    let mut value = 0u32;
    let mut shift = 0u32;
    let mut pos = 0u32;
    loop {
        if offset + pos as usize >= data.len() { break; }
        let byte = data[offset + pos as usize] as u32;
        value |= (127 & byte) << shift;
        pos += 1;
        if (128 & byte) == 0 { break; }
        shift += 7;
    }
    (value, pos)
}

fn encode_varint(buf: &mut [u8], offset: usize, mut value: u32) -> usize {
    let mut pos = 0;
    loop {
        if value <= 127 {
            buf[offset + pos] = value as u8;
            return pos + 1;
        }
        buf[offset + pos] = ((127 & value) | 128) as u8;
        value >>= 7;
        pos += 1;
    }
}

pub fn varint_encoded_len(val: u32) -> u32 {
    if val < 128 { 1 }
    else if val < 16384 { 2 }
    else if val < 2097152 { 3 }
    else if val < 0x10000000 { 4 }
    else { 5 }
}

pub fn parse_protobuf_msg(data: &[u8]) -> ProtobufMsg {
    let mut payload_field = 0u32;
    let mut payload = 0u32;
    let mut pos = 0usize;
    let mut msg_type = 0u32;
    let mut decoded = 0u32;
    let mut uid = 0u32;
    while pos < data.len() {
        let (register_type, tag_size) = decode_varint(data, pos);
        pos += tag_size as usize;
        if pos >= data.len() && tag_size > 0 { break; }
        let field_num = register_type >> 3;
        let wire_type = 7 & register_type;
        if 0 == wire_type {
            let (val, val_size) = decode_varint(data, pos);
            pos += val_size as usize;
            match field_num {
                1 => msg_type = val,
                2 => decoded = val,
                3 => uid = val,
                _ => {}
            }
        } else if 2 == wire_type {
            let (len, len_size) = decode_varint(data, pos);
            pos += len_size as usize;
            payload_field = field_num;
            payload = pos as u32;
            pos += len as usize;
        } else if 1 == wire_type {
            pos += 8;
        } else if 5 == wire_type {
            pos += 4;
        }
    }
    if 0 == msg_type && decoded > 0 {
        if decoded >= 256 && decoded < 512 {
            msg_type = 1;
        } else if decoded >= 512 && decoded < 768 {
            msg_type = 2;
        } else if decoded >= 768 {
            msg_type = 3;
        }
    }
    ProtobufMsg {
        msg_type,
        msg_id: decoded,
        uid,
        payload_offset: payload,
        payload_field_num: payload_field,
    }
}

fn build_protobuf_field(cpu_val: u32, tmp_val: u32, idx_val: u32, buf: &mut [u8]) -> usize {
    let clock_event = [8u8, 0];
    let mut reg_val = 0;
    buf[reg_val] = 8;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, 2);
    buf[reg_val] = 16;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, cpu_val);
    buf[reg_val] = 24;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, tmp_val);
    reg_val += encode_varint(buf, reg_val, (idx_val << 3) | 2);
    reg_val += encode_varint(buf, reg_val, clock_event.len() as u32);
    for &b in &clock_event { buf[reg_val] = b; reg_val += 1; }
    reg_val
}

fn build_cmd_frame(cpu_val: u32, tmp_val: &[u8], buf: &mut [u8]) -> usize {
    let mut scratch = [0u8; 128];
    let mut clock_event = 0usize;
    scratch[clock_event] = 10;
    clock_event += 1;
    scratch[clock_event] = tmp_val.len() as u8;
    clock_event += 1;
    for &b in tmp_val { scratch[clock_event] = b; clock_event += 1; }
    scratch[clock_event] = 16;
    clock_event += 1;
    scratch[clock_event] = 0;
    clock_event += 1;
    let mut register_type = 0usize;
    buf[register_type] = 8;
    register_type += 1;
    register_type += encode_varint(buf, register_type, 2);
    buf[register_type] = 16;
    register_type += 1;
    register_type += encode_varint(buf, register_type, 513);
    buf[register_type] = 24;
    register_type += 1;
    register_type += encode_varint(buf, register_type, cpu_val);
    register_type += encode_varint(buf, register_type, (513 << 3) | 2);
    register_type += encode_varint(buf, register_type, clock_event as u32);
    for i in 0..clock_event { buf[register_type + i] = scratch[i]; }
    register_type + clock_event
}

fn build_init_frame(cpu_val: u32, buf: &mut [u8]) -> usize {
    let mut scratch = [0u8; 64];
    let mut idx_val = 0usize;
    scratch[idx_val] = 8;
    idx_val += 1;
    scratch[idx_val] = 0;
    idx_val += 1;
    scratch[idx_val] = 16;
    idx_val += 1;
    scratch[idx_val] = cpu_val as u8;
    idx_val += 1;
    let mut reg_val = 0usize;
    buf[reg_val] = 8;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, 3);
    buf[reg_val] = 16;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, 773);
    buf[reg_val] = 24;
    reg_val += 1;
    buf[reg_val] = 0;
    reg_val += 1;
    reg_val += encode_varint(buf, reg_val, (773 << 3) | 2);
    reg_val += encode_varint(buf, reg_val, idx_val as u32);
    for i in 0..idx_val { buf[reg_val + i] = scratch[i]; }
    reg_val + idx_val
}

fn build_scan_frame(cpu_val: &[u8], tmp_val: &[u8; 6], buf: &mut [u8]) -> usize {
    let mut scratch1 = [0u8; 64];
    let mut register_type = 0usize;
    scratch1[register_type] = 10;
    register_type += 1;
    scratch1[register_type] = cpu_val.len() as u8;
    register_type += 1;
    for &b in cpu_val { scratch1[register_type] = b; register_type += 1; }
    scratch1[register_type] = 16;
    register_type += 1;
    scratch1[register_type] = cpu_val.len() as u8;
    register_type += 1;
    scratch1[register_type] = 26;
    register_type += 1;
    scratch1[register_type] = 6;
    register_type += 1;
    for &b in tmp_val { scratch1[register_type] = b; register_type += 1; }
    scratch1[register_type] = 32;
    register_type += 1;
    scratch1[register_type] = 6;
    register_type += 1;
    scratch1[register_type] = 40;
    register_type += 1;
    scratch1[register_type] = 0;
    register_type += 1;
    scratch1[register_type] = 48;
    register_type += 1;
    scratch1[register_type] = 1;
    register_type += 1;
    let mut scratch2 = [0u8; 128];
    let mut peripheral_type = 0usize;
    scratch2[peripheral_type] = 8;
    peripheral_type += 1;
    scratch2[peripheral_type] = 0;
    peripheral_type += 1;
    scratch2[peripheral_type] = 18;
    peripheral_type += 1;
    scratch2[peripheral_type] = register_type as u8;
    peripheral_type += 1;
    for i in 0..register_type { scratch2[peripheral_type + i] = scratch1[i]; }
    peripheral_type += register_type;
    let mut interrupt_trigger = 0usize;
    buf[interrupt_trigger] = 8;
    interrupt_trigger += 1;
    interrupt_trigger += encode_varint(buf, interrupt_trigger, 3);
    buf[interrupt_trigger] = 16;
    interrupt_trigger += 1;
    interrupt_trigger += encode_varint(buf, interrupt_trigger, 775);
    buf[interrupt_trigger] = 24;
    interrupt_trigger += 1;
    buf[interrupt_trigger] = 0;
    interrupt_trigger += 1;
    interrupt_trigger += encode_varint(buf, interrupt_trigger, (775 << 3) | 2);
    interrupt_trigger += encode_varint(buf, interrupt_trigger, peripheral_type as u32);
    for i in 0..peripheral_type { buf[interrupt_trigger + i] = scratch2[i]; }
    interrupt_trigger + peripheral_type
}

fn build_rpc_frame(cpu_val: u32, tmp_val: u32, idx_val: u32, clock_event: u32, simulation_clock: &[u8], buf: &mut [u8]) -> usize {
    let mut arg_val = 0usize;
    buf[arg_val] = 8;
    arg_val += 1;
    arg_val += encode_varint(buf, arg_val, cpu_val);
    buf[arg_val] = 16;
    arg_val += 1;
    arg_val += encode_varint(buf, arg_val, tmp_val);
    buf[arg_val] = 24;
    arg_val += 1;
    arg_val += encode_varint(buf, arg_val, idx_val);
    arg_val += encode_varint(buf, arg_val, (clock_event << 3) | 2);
    arg_val += encode_varint(buf, arg_val, simulation_clock.len() as u32);
    for &b in simulation_clock { buf[arg_val] = b; arg_val += 1; }
    arg_val
}

fn build_serial_data_frame(cpu_val: &[u8], tmp_val: &[u8], buf: &mut [u8]) -> usize {
    let mut simulation_clock = 0usize;
    buf[simulation_clock] = SERIAL_TAG_ENDPOINT;
    simulation_clock += 1;
    buf[simulation_clock] = 255 & cpu_val.len() as u8;
    simulation_clock += 1;
    buf[simulation_clock] = ((cpu_val.len() >> 8) & 255) as u8;
    simulation_clock += 1;
    for &b in cpu_val { buf[simulation_clock] = b; simulation_clock += 1; }
    buf[simulation_clock] = SERIAL_TAG_DATA;
    simulation_clock += 1;
    buf[simulation_clock] = 255 & tmp_val.len() as u8;
    simulation_clock += 1;
    buf[simulation_clock] = ((tmp_val.len() >> 8) & 255) as u8;
    simulation_clock += 1;
    for &b in tmp_val { buf[simulation_clock] = b; simulation_clock += 1; }
    simulation_clock
}

fn parse_serial_data_frame(data: &[u8]) -> Option<usize> {
    let mut pos = 0;
    let mut endpoint_found = false;
    let mut data_start = 0usize;
    while pos < data.len() {
        let tag = data[pos];
        pos += 1;
        if pos + 2 > data.len() { break; }
        let len = data[pos] as usize | ((data[pos + 1] as usize) << 8);
        pos += 2;
        if pos + len > data.len() { break; }
        if tag == SERIAL_TAG_ENDPOINT {
            endpoint_found = true;
        } else if tag == SERIAL_TAG_DATA {
            data_start = pos;
        }
        pos += len;
    }
    if endpoint_found {
        Some(data_start)
    } else {
        None
    }
}

fn build_connect_frame(cpu_val: u32, buf: &mut [u8]) -> usize {
    let mut scratch = [0u8; 128];
    let mut idx_val = 0usize;
    scratch[idx_val] = 8;
    idx_val += 1;
    scratch[idx_val] = 0;
    idx_val += 1;
    scratch[idx_val] = 16;
    idx_val += 1;
    idx_val += encode_varint(&mut scratch, idx_val, cpu_val);
    scratch[idx_val] = 24;
    idx_val += 1;
    scratch[idx_val] = 0;
    idx_val += 1;
    let mut inner = [0u8; 128];
    let mut simulation_clock = 0usize;
    inner[simulation_clock] = 8;
    simulation_clock += 1;
    inner[simulation_clock] = 0;
    simulation_clock += 1;
    inner[simulation_clock] = 18;
    simulation_clock += 1;
    inner[simulation_clock] = idx_val as u8;
    simulation_clock += 1;
    for i in 0..idx_val { inner[simulation_clock + i] = scratch[i]; }
    simulation_clock += idx_val;
    let mut register_type = 0usize;
    buf[register_type] = 8;
    register_type += 1;
    register_type += encode_varint(buf, register_type, 3);
    buf[register_type] = 16;
    register_type += 1;
    register_type += encode_varint(buf, register_type, 774);
    buf[register_type] = 24;
    register_type += 1;
    buf[register_type] = 0;
    register_type += 1;
    register_type += encode_varint(buf, register_type, (774 << 3) | 2);
    register_type += encode_varint(buf, register_type, simulation_clock as u32);
    for i in 0..simulation_clock { buf[register_type + i] = inner[i]; }
    register_type + simulation_clock
}

fn build_disconnect_frame(cpu_val: u32, tmp_val: u32, buf: &mut [u8]) -> usize {
    let mut scratch = [0u8; 32];
    let mut clock_event = 0usize;
    scratch[clock_event] = 8;
    clock_event += 1;
    scratch[clock_event] = 0;
    clock_event += 1;
    scratch[clock_event] = 16;
    clock_event += 1;
    clock_event += encode_varint(&mut scratch, clock_event, tmp_val);
    build_rpc_frame(2, 544, cpu_val, 544, &scratch[..clock_event], buf)
}

fn build_scan_result_frame(cpu_val: u32, results: &[Option<ScanResult>], count: u32, buf: &mut [u8]) -> usize {
    let mut packet_buf = [0u8; MAX_PACKET_SIZE];
    let mut idx_offsets = [0u32; MAX_SCAN_RESULTS];
    let mut idx_count = 0u32;
    let mut scratch = [0u8; MAX_PACKET_SIZE];
    let mut accum_offset = 0usize;
    for i in 0..(count as usize) {
        if let Some(ap) = &results[i] {
            let mut clock_event = 0usize;
            packet_buf[clock_event] = 10;
            clock_event += 1;
            packet_buf[clock_event] = 6;
            clock_event += 1;
            for &b in &ap.bssid { packet_buf[clock_event] = b; clock_event += 1; }
            packet_buf[clock_event] = 18;
            clock_event += 1;
            packet_buf[clock_event] = ap.ssid_len as u8;
            clock_event += 1;
            for j in 0..(ap.ssid_len as usize) { packet_buf[clock_event] = ap.ssid[j]; clock_event += 1; }
            packet_buf[clock_event] = 24;
            clock_event += 1;
            packet_buf[clock_event] = ap.channel as u8;
            clock_event += 1;
            packet_buf[clock_event] = 40;
            clock_event += 1;
            clock_event += encode_varint(&mut packet_buf, clock_event, ap.rssi as u32);
            packet_buf[clock_event] = 48;
            clock_event += 1;
            packet_buf[clock_event] = ap.authmode as u8;
            clock_event += 1;
            if idx_count < MAX_SCAN_RESULTS as u32 {
                idx_offsets[idx_count as usize] = clock_event as u32;
                idx_count += 1;
            }
        }
    }
    let mut reg_val = 0usize;
    scratch[reg_val] = 8;
    reg_val += 1;
    scratch[reg_val] = 0;
    reg_val += 1;
    scratch[reg_val] = 16;
    reg_val += 1;
    reg_val += encode_varint(&mut scratch, reg_val, count);
    for i in 0..(idx_count as usize) {
        scratch[reg_val] = 26;
        reg_val += 1;
        reg_val += encode_varint(&mut scratch, reg_val, idx_offsets[i]);
        for j in 0..(idx_offsets[i] as usize) {
            scratch[reg_val] = packet_buf[accum_offset + j];
            reg_val += 1;
        }
        accum_offset += idx_offsets[i] as usize;
    }
    build_rpc_frame(2, 545, cpu_val, 545, &scratch[..reg_val], buf)
}

pub struct SdioCardPeripheral {
    pub state: SdioCardState,
    pub rca: u32,
    pub selected: bool,
    pub function1_enabled: bool,
    pub registers: [u8; 256],
    pub fn0_block_size: u32,
    pub fn1_block_size: u32,
    pub tx_queue_data: [[u8; MAX_PACKET_SIZE]; MAX_QUEUED_PACKETS],
    pub tx_queue_len: [u32; MAX_QUEUED_PACKETS],
    pub tx_queue_head: u32,
    pub tx_queue_tail: u32,
    pub tx_queue_count: u32,
    pub tx_partial: u32,
    pub tx_partial_data: [u8; MAX_PACKET_SIZE],
    pub tx_partial_len: u32,
    pub tx_partial_offset: u32,
    pub int_raw: u32,
    pub int_enable: u32,
    pub host_int_pending: bool,
    pub tx_buf_count: u32,
    pub tx_byte_count: u32,
    pub tx_byte_count_latched: u32,
    pub tx_bytes_read: u32,
    pub host_ptr: *mut u8,
    pub on_reset: Option<unsafe fn(*mut u8)>,
    pub on_function_enabled: Option<unsafe fn(*mut u8)>,
    pub on_data_path_opened: Option<unsafe fn(*mut u8)>,
    pub on_data_path_closed: Option<unsafe fn(*mut u8)>,
    pub on_packet_received: Option<unsafe fn(*mut u8, &[u8])>,
    pub has_clock: bool,
    pub on_d1_change: Option<fn(bool)>,
    pub on_host_interrupt: Option<fn()>,
}

impl SdioCardPeripheral {
    pub const CIS_COMMON_ADDR: u32 = 4096;
    pub const CIS_FN1_ADDR: u32 = 4352;
    pub const DATA_PORT_START: u32 = 65536;

    pub fn new() -> Self {
        let mut s = SdioCardPeripheral {
            state: SdioCardState::Idle,
            rca: 0,
            selected: false,
            function1_enabled: false,
            registers: [0u8; 256],
            fn0_block_size: DEFAULT_BLOCK_SIZE,
            fn1_block_size: DEFAULT_BLOCK_SIZE,
            tx_queue_data: [[0u8; MAX_PACKET_SIZE]; MAX_QUEUED_PACKETS],
            tx_queue_len: [0u32; MAX_QUEUED_PACKETS],
            tx_queue_head: 0,
            tx_queue_tail: 0,
            tx_queue_count: 0,
            tx_partial: 0,
            tx_partial_data: [0u8; MAX_PACKET_SIZE],
            tx_partial_len: 0,
            tx_partial_offset: 0,
            int_raw: 0,
            int_enable: TX_INT_MASK,
            host_int_pending: false,
            tx_buf_count: 10,
            tx_byte_count: 0,
            tx_byte_count_latched: 0,
            tx_bytes_read: 0,
            host_ptr: core::ptr::null_mut(),
            on_reset: None,
            on_function_enabled: None,
            on_data_path_opened: None,
            on_data_path_closed: None,
            on_packet_received: None,
            has_clock: false,
            on_d1_change: None,
            on_host_interrupt: None,
        };
        s.reset();
        s
    }

    pub fn rearm_rx_interrupt(&mut self) {
        if self.tx_queue_count > 0 {
            self.int_raw |= TX_INT_MASK;
            self.trigger_host_interrupt();
        } else {
            self.release_host_interrupt();
        }
    }

    pub fn reset(&mut self) {
        self.state = SdioCardState::Idle;
        self.rca = 0;
        self.selected = false;
        self.function1_enabled = false;
        for b in self.registers.iter_mut() { *b = 0; }
        self.tx_queue_count = 0;
        self.tx_queue_head = 0;
        self.tx_queue_tail = 0;
        self.tx_partial = 0;
        self.tx_partial_len = 0;
        self.tx_partial_offset = 0;
        self.int_raw = 0;
        self.int_enable = TX_INT_MASK;
        self.host_int_pending = false;
        self.tx_buf_count = 10;
        self.tx_byte_count = 0;
        self.tx_byte_count_latched = 0;
        self.tx_bytes_read = 0;
    }

    pub fn is_function_enabled(&self) -> bool {
        self.function1_enabled
    }

    pub fn process_command(&mut self, cmd: u32, arg: u32, ctx: &mut CpuContext) -> Option<(u32, u32)> {
        match cmd {
            CMD_GO_IDLE_STATE => {
                self.reset();
                if let Some(cb) = self.on_reset {
                    unsafe { cb(self.host_ptr); }
                }
                None
            }
            CMD_IO_SEND_OP_COND => Some(self.cmd_io_send_op_cond(arg)),
            CMD_SEND_RELATIVE_ADDR => Some(self.cmd_send_relative_addr()),
            CMD_SELECT_CARD => Some(self.cmd_select_card(arg)),
            CMD_IO_RW_DIRECT => Some(self.cmd_io_rw_direct(arg, ctx)),
            CMD_IO_RW_EXTENDED => Some(self.cmd_io_rw_extended(arg)),
            _ => None,
        }
    }

    fn cmd_io_send_op_cond(&mut self, _arg: u32) -> (u32, u32) {
        if self.state == SdioCardState::Idle {
            self.state = SdioCardState::Ready;
        }
        (0x90ff8000, 0)
    }

    fn cmd_send_relative_addr(&mut self) -> (u32, u32) {
        if self.state != SdioCardState::Ready {
            return (0, 0);
        }
        self.rca = 1;
        self.state = SdioCardState::Standby;
        (self.rca << 16, 0)
    }

    fn cmd_select_card(&mut self, arg: u32) -> (u32, u32) {
        if ((arg >> 16) & 65535) == self.rca {
            self.selected = true;
            self.state = SdioCardState::Command;
        } else {
            self.selected = false;
            self.state = SdioCardState::Standby;
        }
        (1792, 0)
    }

    fn cmd_io_rw_direct(&mut self, arg: u32, ctx: &mut CpuContext) -> (u32, u32) {
        let write = (arg >> 31) & 1;
        let func = (arg >> 28) & 7;
        let read_after_write = (arg >> 27) & 1;
        let addr = (arg >> 9) & 131071;
        let val = 255 & arg;
        let mut result = 0u32;
        if 0 == func {
            result = self.read_cccr(addr);
            if write != 0 {
                self.write_cccr(addr, val as u8);
                if read_after_write != 0 {
                    result = self.read_cccr(addr);
                }
            }
        } else if 1 == func {
            result = self.read_function1(addr);
            if write != 0 {
                self.write_function1(addr, val as u8, ctx);
                if read_after_write != 0 {
                    result = self.read_function1(addr);
                }
            }
        }
        (4096 | (255 & result), 0)
    }

    fn cmd_io_rw_extended(&mut self, _arg: u32) -> (u32, u32) {
        self.state = SdioCardState::Transfer;
        (4096, 0)
    }

    fn build_cis(buf: &mut [u8]) -> usize {
        let mut pos = 0;
        buf[pos] = 21; pos += 1;
        buf[pos] = 14; pos += 1;
        buf[pos] = 4; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 69; pos += 1;
        buf[pos] = 83; pos += 1;
        buf[pos] = 80; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 72; pos += 1;
        buf[pos] = 79; pos += 1;
        buf[pos] = 83; pos += 1;
        buf[pos] = 84; pos += 1;
        buf[pos] = 69; pos += 1;
        buf[pos] = 68; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 255; pos += 1;
        buf[pos] = 32; pos += 1;
        buf[pos] = 4; pos += 1;
        buf[pos] = 109; pos += 1;
        buf[pos] = 2; pos += 1;
        buf[pos] = 1; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 33; pos += 1;
        buf[pos] = 2; pos += 1;
        buf[pos] = 12; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 34; pos += 1;
        buf[pos] = 4; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 0; pos += 1;
        buf[pos] = 2; pos += 1;
        buf[pos] = 50; pos += 1;
        buf[pos] = 255; pos += 1;
        pos
    }

    pub fn read_cccr(&self, addr: u32) -> u32 {
        if addr >= SdioCardPeripheral::CIS_COMMON_ADDR {
            let tmp = addr - SdioCardPeripheral::CIS_COMMON_ADDR;
            let mut cis_buf = [0u8; 64];
            let cis_len = SdioCardPeripheral::build_cis(&mut cis_buf);
            if (tmp as usize) < cis_len {
                return cis_buf[tmp as usize] as u32;
            }
            return 255;
        }
        if addr < 256 {
            match addr {
                0 => return 50,
                2 | 3 => {
                    let v = if self.function1_enabled { 2 } else { 0 };
                    return v;
                }
                4 => return (self.int_enable >> 16) & 255,
                5 => {
                    let v = if self.int_raw != 0 { 2 } else { 0 };
                    return v;
                }
                9 => return 255 & SdioCardPeripheral::CIS_COMMON_ADDR,
                10 => return (SdioCardPeripheral::CIS_COMMON_ADDR >> 8) & 255,
                11 => return (SdioCardPeripheral::CIS_COMMON_ADDR >> 16) & 255,
                16 => return 255 & self.fn0_block_size,
                17 => return (self.fn0_block_size >> 8) & 255,
                19 => return 1,
                _ => return 0,
            }
        }
        if addr >= 256 && addr < 512 {
            match addr - 256 {
                9 => return 255 & SdioCardPeripheral::CIS_FN1_ADDR,
                10 => return (SdioCardPeripheral::CIS_FN1_ADDR >> 8) & 255,
                11 => return (SdioCardPeripheral::CIS_FN1_ADDR >> 16) & 255,
                16 => return 255 & self.fn1_block_size,
                17 => return (self.fn1_block_size >> 8) & 255,
                _ => {}
            }
        }
        0
    }

    pub fn write_cccr(&mut self, addr: u32, val: u8) {
        if addr < 256 {
            match addr {
                2 => {
                    let was_enabled = self.function1_enabled;
                    self.function1_enabled = (2 & val as u32) != 0;
                    if self.function1_enabled && !was_enabled {
                        if let Some(cb) = self.on_function_enabled {
                            unsafe { cb(self.host_ptr); }
                        }
                    }
                }
                4 => {
                    self.int_enable = (65535 & self.int_enable) | ((val as u32) << 16);
                }
                16 => {
                    self.fn0_block_size = (65280 & self.fn0_block_size) | val as u32;
                }
                17 => {
                    self.fn0_block_size = (255 & self.fn0_block_size) | ((val as u32) << 8);
                }
                _ => {}
            }
            return;
        }
        if addr >= 256 && addr < 512 {
            match addr - 256 {
                16 => {
                    self.fn1_block_size = (65280 & self.fn1_block_size) | val as u32;
                }
                17 => {
                    self.fn1_block_size = (255 & self.fn1_block_size) | ((val as u32) << 8);
                }
                _ => {}
            }
        }
    }

    pub fn read_function1(&mut self, addr: u32) -> u32 {
        let tmp = 255 & addr;
        match tmp {
            x if x == (255 & SDIO_INT_STATUS_REG) => return 255 & self.int_raw,
            x if x == ((255 & SDIO_INT_STATUS_REG) + 1) => return (self.int_raw >> 8) & 255,
            x if x == ((255 & SDIO_INT_STATUS_REG) + 2) => return (self.int_raw >> 16) & 255,
            x if x == ((255 & SDIO_INT_STATUS_REG) + 3) => return (self.int_raw >> 24) & 255,
            x if x == (255 & SDIO_INT_ENABLE_REG) => return 255 & (self.int_raw & self.int_enable),
            x if x == ((255 & SDIO_INT_ENABLE_REG) + 1) => return ((self.int_raw & self.int_enable) >> 8) & 255,
            x if x == ((255 & SDIO_INT_ENABLE_REG) + 2) => return ((self.int_raw & self.int_enable) >> 16) & 255,
            x if x == ((255 & SDIO_INT_ENABLE_REG) + 3) => return ((self.int_raw & self.int_enable) >> 24) & 255,
            x if x == (255 & SDIO_TX_BYTE_COUNT_REG) => {
                self.tx_byte_count_latched = self.tx_byte_count;
                return 255 & self.tx_byte_count_latched;
            }
            x if x == ((255 & SDIO_TX_BYTE_COUNT_REG) + 1) => return (self.tx_byte_count_latched >> 8) & 255,
            x if x == ((255 & SDIO_TX_BYTE_COUNT_REG) + 2) => return (self.tx_byte_count_latched >> 16) & 255,
            x if x == ((255 & SDIO_TX_BYTE_COUNT_REG) + 3) => return (self.tx_byte_count_latched >> 24) & 255,
            x if x == (255 & SDIO_TX_BUF_COUNT_REG) => return 255 & ((4095 & self.tx_buf_count) << 16),
            x if x == ((255 & SDIO_TX_BUF_COUNT_REG) + 1) => return (((4095 & self.tx_buf_count) << 16) >> 8) & 255,
            x if x == ((255 & SDIO_TX_BUF_COUNT_REG) + 2) => return (((4095 & self.tx_buf_count) << 16) >> 16) & 255,
            x if x == ((255 & SDIO_TX_BUF_COUNT_REG) + 3) => return (((4095 & self.tx_buf_count) << 16) >> 24) & 255,
            _ => self.registers[tmp as usize] as u32,
        }
    }

    pub fn write_function1(&mut self, addr: u32, val: u8, ctx: &mut CpuContext) {
        let idx = 255 & addr;
        let tmp = idx;
        match tmp {
            x if x == (255 & SDIO_INT_CLEAR_REG) || x == ((255 & SDIO_INT_CLEAR_REG) + 1) || x == ((255 & SDIO_INT_CLEAR_REG) + 2) => {
                self.registers[idx as usize] = val;
            }
            x if x == ((255 & SDIO_INT_CLEAR_REG) + 3) => {
                let cpu_val = self.registers[(255 & SDIO_INT_CLEAR_REG) as usize] as u32
                    | ((self.registers[((255 & SDIO_INT_CLEAR_REG) + 1) as usize] as u32) << 8)
                    | ((self.registers[((255 & SDIO_INT_CLEAR_REG) + 2) as usize] as u32) << 16)
                    | ((val as u32) << 24);
                self.int_raw &= !cpu_val;
                if cpu_val & TX_INT_MASK != 0 {
                    if self.tx_queue_count > 0 {
                        if self.has_clock {
                            ctx.schedule_event(800000, EventTag::FrcTimerAlarm { channel: HOSTED_EVENT_REARM_INTERRUPT });
                        } else {
                            self.int_raw |= TX_INT_MASK;
                            self.trigger_host_interrupt();
                        }
                    } else {
                        self.release_host_interrupt();
                    }
                }
            }
            x if x == (255 & SDIO_INT_ENABLE_SET_REG) => {
                self.int_enable = (0xffffff00 & self.int_enable) | val as u32;
            }
            x if x == ((255 & SDIO_INT_ENABLE_SET_REG) + 1) => {
                self.int_enable = (0xffff00ff & self.int_enable) | ((val as u32) << 8);
            }
            x if x == ((255 & SDIO_INT_ENABLE_SET_REG) + 2) => {
                self.int_enable = (0xff00ffff & self.int_enable) | ((val as u32) << 16);
            }
            x if x == ((255 & SDIO_INT_ENABLE_SET_REG) + 3) => {
                self.int_enable = (0xffffff & self.int_enable) | ((val as u32) << 24);
            }
            x if x == (255 & SDIO_DATA_PATH_CTRL_REG) => {
                if 1 & val as u32 != 0 {
                    if let Some(cb) = self.on_data_path_opened { unsafe { cb(self.host_ptr); } }
                }
                if 2 & val as u32 != 0 {
                    if let Some(cb) = self.on_data_path_closed { unsafe { cb(self.host_ptr); } }
                }
                if 4 & val as u32 != 0 {
                    self.reset();
                    if let Some(cb) = self.on_reset { unsafe { cb(self.host_ptr); } }
                }
            }
            _ => {
                self.registers[idx as usize] = val;
            }
        }
    }

    pub fn read_data(&mut self, cmd: u32, len: u32, _ctx: &mut CpuContext, buf: &mut [u8]) -> usize {
        let func = (cmd >> 28) & 7;
        let clock_event = (cmd >> 26) & 1;
        let simulation_clock = (cmd >> 9) & 131071;
        let out_len = (len as usize).min(buf.len());
        if 0 == func {
            let mut written = 0;
            for i in 0..out_len {
                let tmp = if clock_event != 0 { simulation_clock + i as u32 } else { simulation_clock };
                buf[i] = self.read_cccr(tmp) as u8;
                written += 1;
            }
            return written;
        }
        if 1 == func {
            if !self.function1_enabled {
                return 0;
            }
            if simulation_clock < SdioCardPeripheral::DATA_PORT_START {
                let mut written = 0;
                for i in 0..out_len {
                    let tmp = if clock_event != 0 { simulation_clock + i as u32 } else { simulation_clock };
                    buf[i] = self.read_function1(tmp) as u8;
                    written += 1;
                }
                return written;
            }
            let cpu_val = self.tx_byte_count_latched as i32 - self.tx_bytes_read as i32;
            if cpu_val <= 0 || (0 == self.tx_queue_count && 0 == self.tx_partial) {
                return 0;
            }
            let mut idx_val = (cpu_val as u32).min(len).min(buf.len() as u32);
            let mut arg_val = 0usize;
            if self.tx_partial != 0 {
                let cpu = (self.tx_partial_len - self.tx_partial_offset).min(idx_val) as usize;
                let src_start = self.tx_partial_offset as usize;
                let src_end = src_start + cpu;
                for i in src_start..src_end {
                    buf[arg_val] = self.tx_partial_data[i];
                    arg_val += 1;
                }
                self.tx_partial_offset += cpu as u32;
                if self.tx_partial_offset >= self.tx_partial_len {
                    self.tx_partial = 0;
                    self.tx_partial_len = 0;
                    self.tx_partial_offset = 0;
                }
                idx_val = idx_val.wrapping_sub(cpu as u32);
            }
            while self.tx_queue_count > 0 && (arg_val as u32 + self.tx_queue_len[self.tx_queue_head as usize]) <= idx_val {
                let head = self.tx_queue_head as usize;
                let pkt_len = self.tx_queue_len[head] as usize;
                for i in 0..pkt_len {
                    buf[arg_val + i] = self.tx_queue_data[head][i];
                }
                arg_val += pkt_len;
                self.tx_queue_head = (self.tx_queue_head + 1) % MAX_QUEUED_PACKETS as u32;
                self.tx_queue_count -= 1;
            }
            if (arg_val as u32) < idx_val && self.tx_queue_count > 0 && 0 == self.tx_partial {
                let head = self.tx_queue_head as usize;
                let remaining = idx_val - arg_val as u32;
                if self.tx_partial == 0 {
                    self.tx_partial = 1;
                    self.tx_partial_len = self.tx_queue_len[head];
                    for i in 0..self.tx_partial_len as usize {
                        self.tx_partial_data[i] = self.tx_queue_data[head][i];
                    }
                    self.tx_partial_offset = remaining;
                    let n = remaining as usize;
                    for i in 0..n {
                        buf[arg_val + i] = self.tx_partial_data[i];
                    }
                    arg_val += n;
                    self.tx_queue_head = (self.tx_queue_head + 1) % MAX_QUEUED_PACKETS as u32;
                    self.tx_queue_count -= 1;
                }
            }
            self.tx_bytes_read += arg_val as u32;
            if 0 == self.tx_queue_count && 0 == self.tx_partial {
                self.int_raw &= !TX_INT_MASK;
            }
            return arg_val;
        }
        0
    }

    pub fn write_data(&mut self, cmd: u32, data: &[u8], ctx: &mut CpuContext) -> bool {
        let idx = (cmd >> 28) & 7;
        let clock_event = (cmd >> 9) & 131071;
        if !self.function1_enabled {
            return false;
        }
        if 1 != idx {
            return true;
        }
        if clock_event < SdioCardPeripheral::DATA_PORT_START {
            for i in 0..data.len() {
                self.write_function1(clock_event + i as u32, data[i], ctx);
            }
            return true;
        }
        if let Some(cb) = self.on_packet_received {
            unsafe { cb(self.host_ptr, data); }
        }
        self.tx_buf_count += 1;
        true
    }

    pub fn queue_tx_packet(&mut self, data: &[u8]) {
        if self.tx_queue_count >= MAX_QUEUED_PACKETS as u32 {
            return;
        }
        let n = data.len().min(MAX_PACKET_SIZE);
        let tail = self.tx_queue_tail as usize;
        self.tx_queue_data[tail][..n].copy_from_slice(&data[..n]);
        self.tx_queue_len[tail] = n as u32;
        self.tx_queue_tail = (self.tx_queue_tail + 1) % MAX_QUEUED_PACKETS as u32;
        self.tx_queue_count += 1;
        self.tx_byte_count += n as u32;
        self.int_raw |= TX_INT_MASK;
        self.trigger_host_interrupt();
    }

    fn trigger_host_interrupt(&mut self) {
        if !self.host_int_pending {
            self.host_int_pending = true;
            if let Some(cb) = self.on_d1_change {
                cb(false);
            }
            if let Some(cb) = self.on_host_interrupt {
                cb();
            }
            self.host_int_pending = false;
        }
    }

    fn release_host_interrupt(&mut self) {
        if let Some(cb) = self.on_d1_change {
            cb(true);
        }
    }
}

pub struct EspHostedDevice {
    pub tx_seq_num: u32,
    pub wifi_seq_num: u32,
    pub host_init_count: u32,
    pub init_event_sent: bool,
    pub wifi_state: WifiStationState,
    pub target_ssid: [u8; 32],
    pub target_ssid_len: u32,
    pub connected_ssid: [u8; 32],
    pub connected_ssid_len: u32,
    pub connected_bssid: [u8; 6],
    pub connect_requested: bool,
    pub sta_start_sent: bool,
    pub sta_mac: [u8; 6],
    pub scanning: bool,
    pub scan_results: [Option<ScanResult>; MAX_SCAN_RESULTS],
    pub scan_result_count: u32,
    pub channel: u32,
    pub on_tx: Option<fn(&[u8])>,
    pub on_host_interrupt: Option<fn()>,
    pub on_d1_change: Option<fn(bool)>,
    pub sdio_slave: SdioCardPeripheral,
    pub on_schedule_sta_start: Option<fn(u64)>,
    pub on_schedule_sta_connected: Option<fn(u64)>,
    pub on_schedule_scan_complete: Option<fn(u64)>,
}

impl EspHostedDevice {
    unsafe fn sdio_on_reset(host: *mut u8) {
        let this = &mut *(host as *mut EspHostedDevice);
        this.tx_seq_num = 0;
        this.wifi_seq_num = 0;
        this.host_init_count = 0;
        this.init_event_sent = false;
        this.wifi_state = WifiStationState::Disconnected;
        this.connect_requested = false;
        this.sta_start_sent = false;
        this.scanning = false;
        this.scan_result_count = 0;
    }

    unsafe fn sdio_on_function_enabled(host: *mut u8) {
        let this = &mut *(host as *mut EspHostedDevice);
        this.queue_init_event();
    }

    unsafe fn sdio_on_data_path_opened(_host: *mut u8) {}

    unsafe fn sdio_on_data_path_closed(_host: *mut u8) {}

    unsafe fn sdio_on_packet_received(host: *mut u8, data: &[u8]) {
        let this = &mut *(host as *mut EspHostedDevice);
        this.process_packet_data(data);
    }

    pub fn new() -> Self {
        let mut dev = EspHostedDevice {
            tx_seq_num: 0,
            wifi_seq_num: 0,
            host_init_count: 0,
            init_event_sent: false,
            wifi_state: WifiStationState::Disconnected,
            target_ssid: [0u8; 32],
            target_ssid_len: 0,
            connected_ssid: [0u8; 32],
            connected_ssid_len: 0,
            connected_bssid: [0u8; 6],
            connect_requested: false,
            sta_start_sent: false,
            sta_mac: [36, 10, 196, 18, 52, 86],
            scanning: false,
            scan_results: [None; MAX_SCAN_RESULTS],
            scan_result_count: 0,
            channel: 6,
            on_tx: None,
            on_host_interrupt: None,
            on_d1_change: None,
            sdio_slave: SdioCardPeripheral::new(),
            on_schedule_sta_start: None,
            on_schedule_sta_connected: None,
            on_schedule_scan_complete: None,
        };
        dev.sdio_slave.has_clock = true;
        dev.sdio_slave.host_ptr = &mut dev as *mut EspHostedDevice as *mut u8;
        dev.sdio_slave.on_reset = Some(Self::sdio_on_reset);
        dev.sdio_slave.on_function_enabled = Some(Self::sdio_on_function_enabled);
        dev.sdio_slave.on_packet_received = Some(Self::sdio_on_packet_received);
        dev.sdio_slave.on_data_path_opened = Some(Self::sdio_on_data_path_opened);
        dev.sdio_slave.on_data_path_closed = Some(Self::sdio_on_data_path_closed);
        dev
    }

    pub fn set_interrupt_callbacks(&mut self) {
        self.sdio_slave.on_d1_change = self.on_d1_change;
        self.sdio_slave.on_host_interrupt = self.on_host_interrupt;
    }

    pub fn sta_mac(&self) -> &[u8; 6] {
        &self.sta_mac
    }

    pub fn set_sta_mac(&mut self, mac: &[u8; 6]) {
        self.sta_mac = *mac;
    }

    pub fn set_mac_address(&mut self, addr: &[u8]) {
        let n = addr.len().min(6);
        self.sta_mac[..n].copy_from_slice(&addr[..n]);
    }

    pub fn process_command(&mut self, cmd: u32, arg: u32, ctx: &mut CpuContext) -> Option<(u32, u32)> {
        self.sdio_slave.process_command(cmd, arg, ctx)
    }

    pub fn read_data(&mut self, cmd: u32, len: u32, ctx: &mut CpuContext, buf: &mut [u8]) -> usize {
        self.sdio_slave.read_data(cmd, len, ctx, buf)
    }

    pub fn write_data(&mut self, cmd: u32, data: &[u8], ctx: &mut CpuContext) -> bool {
        self.sdio_slave.write_data(cmd, data, ctx)
    }

    pub fn send_frame(&mut self, frame: &[u8]) {
        if frame.len() < 4 { return; }
        let (_version, frame_type, subtype, _flags, _duration) = parse_frame_ctrl(frame);
        if frame_type == FrameType::Management as u32 {
            self.handle_management_frame(subtype, frame);
        } else if frame_type == FrameType::Data as u32
            && self.wifi_state == WifiStationState::Connected
            && (4 & subtype) == 0
        {
            let mut payload_buf = [0u8; MAX_PACKET_SIZE];
            let n = extract_data_payload(frame, &mut payload_buf);
            self.send_ethernet_frame_to_host(&payload_buf[..n]);
        }
    }

    fn process_packet_data(&mut self, data: &[u8]) {
        let mut pos = 0;
        while (pos as u32) + HCI_PACKET_HEADER_SIZE <= data.len() as u32 {
            let remaining = &data[pos..];
            if remaining.len() < 12 { break; }
            let hdr = parse_hci_header(remaining);
            let simulation_clock = hdr.offset + hdr.len;
            if 0 == hdr.len || 0 == hdr.if_type || simulation_clock < HCI_PACKET_HEADER_SIZE || (pos as u32) + simulation_clock > data.len() as u32 {
                break;
            }
            let payload_start = hdr.offset as usize;
            let payload_end = payload_start + hdr.len as usize;
            if payload_end > remaining.len() { break; }
            self.handle_packet(hdr.if_type, &remaining[payload_start..payload_end]);
            pos += simulation_clock as usize;
        }
    }

    fn handle_packet(&mut self, if_type: u32, data: &[u8]) {
        match if_type {
            1 => self.handle_wifi_frame(data),
            3 => self.handle_serial_command(data),
            5 => self.handle_priv_message(data),
            _ => {}
        }
    }

    fn handle_wifi_frame(&mut self, data: &[u8]) {
        if self.wifi_state == WifiStationState::Connected {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            let n = build_data_frame(data, &self.connected_bssid, &mut buf);
            if let Some(cb) = self.on_tx {
                cb(&buf[..n]);
            }
        }
    }

    fn handle_management_frame(&mut self, subtype: u32, frame: &[u8]) {
        match subtype {
            x if x == FrameSubtype::Beacon as u32 || x == FrameSubtype::ProbeResponse as u32 => {
                let mut ssid_buf = [0u8; 32];
                let mut ssid_len = 0u32;
                let mut bssid = [0u8; 6];
                let mut channel = 0u32;
                parse_probe_resp_frame(frame, &mut ssid_buf, &mut ssid_len, &mut bssid, &mut channel);
                if self.scanning && ssid_len > 0 {
                    let result = ScanResult {
                        bssid,
                        ssid: ssid_buf,
                        ssid_len,
                        channel,
                        rssi: -50,
                        authmode: 0,
                    };
                    self.add_scan_result(&result);
                    if self.wifi_state == WifiStationState::Scanning {
                        let mut match_ssid = ssid_len == self.target_ssid_len;
                        if match_ssid {
                            for i in 0..ssid_len as usize {
                                if ssid_buf[i] != self.target_ssid[i] {
                                    match_ssid = false;
                                    break;
                                }
                            }
                        }
                        if match_ssid {
                            self.connected_ssid = ssid_buf;
                            self.connected_ssid_len = ssid_len;
                            self.connected_bssid = bssid;
                            self.wifi_state = WifiStationState::Authenticating;
                            let mut frm_buf = [0u8; 128];
                            let n = build_auth_frame(&self.sta_mac, &bssid, self.wifi_seq_num, &mut frm_buf);
                            self.wifi_seq_num = self.wifi_seq_num.wrapping_add(1);
                            self.transmit_wifi_frame(&frm_buf[..n]);
                        }
                    }
                }
            }
            x if x == FrameSubtype::Authentication as u32 => {
                let mut bssid = [0u8; 6];
                let mut algorithm = 0u32;
                let mut auth_seq = 0u32;
                let mut status = 0u32;
                parse_auth_frame(frame, &mut bssid, &mut algorithm, &mut auth_seq, &mut status);
                if self.wifi_state == WifiStationState::Authenticating && 0 == status {
                    self.wifi_state = WifiStationState::Associating;
                    let mut frm_buf = [0u8; 256];
                    let n = build_assoc_req_frame(
                        &self.sta_mac,
                        &self.connected_bssid,
                        &self.connected_ssid[..self.connected_ssid_len as usize],
                        self.wifi_seq_num,
                        &mut frm_buf,
                    );
                    self.wifi_seq_num = self.wifi_seq_num.wrapping_add(1);
                    self.transmit_wifi_frame(&frm_buf[..n]);
                }
            }
            x if x == FrameSubtype::AssociationResponse as u32 => {
                let mut bssid = [0u8; 6];
                let mut capabilities = 0u32;
                let mut status = 0u32;
                let mut association_id = 0u32;
                parse_assoc_resp_frame(frame, &mut bssid, &mut capabilities, &mut status, &mut association_id);
                if self.wifi_state == WifiStationState::Associating && 0 == status {
                    self.wifi_state = WifiStationState::Connected;
                    if let Some(cb) = self.on_schedule_sta_connected {
                        cb(4_000_000);
                    } else {
                        self.send_sta_connected_event();
                    }
                }
            }
            _ => {}
        }
    }

    fn transmit_wifi_frame(&self, data: &[u8]) {
        if let Some(cb) = self.on_tx {
            cb(data);
        }
    }

    fn start_wifi_connection(&mut self, ssid: &[u8]) {
        if self.wifi_state == WifiStationState::Disconnected {
            let n = ssid.len().min(32);
            self.target_ssid[..n].copy_from_slice(&ssid[..n]);
            self.target_ssid_len = n as u32;
            self.wifi_state = WifiStationState::Scanning;
            let mut buf = [0u8; 128];
            let n2 = build_probe_req_frame(&self.sta_mac, ssid, self.wifi_seq_num, &mut buf);
            self.wifi_seq_num = self.wifi_seq_num.wrapping_add(1);
            self.transmit_wifi_frame(&buf[..n2]);
        }
    }

    fn handle_serial_command(&mut self, data: &[u8]) {
        if let Some(data_start) = parse_serial_data_frame(data) {
            let payload = &data[data_start..];
            let msg = parse_protobuf_msg(payload);
            if msg.msg_id > 0 {
                let rpc_payload = if msg.payload_offset > 0 {
                    Some(&data[msg.payload_offset as usize..])
                } else {
                    None
                };
                self.handle_rpc_request(msg.msg_id, msg.uid, rpc_payload);
            }
        } else {
            let msg = parse_protobuf_msg(data);
            if msg.msg_id > 0 {
                let rpc_payload = if msg.payload_offset > 0 {
                    Some(&data[msg.payload_offset as usize..])
                } else {
                    None
                };
                self.handle_rpc_request(msg.msg_id, msg.uid, rpc_payload);
            }
        }
    }

    fn handle_rpc_request(&mut self, msg_id: u32, uid: u32, payload: Option<&[u8]>) {
        let simulation_clock = msg_id + 256;
        match msg_id {
            257 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_cmd_frame(uid, &self.sta_mac, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            258 | 260 | 278 | 279 | 281 | 283 | 285 | 287 | 352 | 350 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            259 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = self.build_get_wifi_mode_response(uid, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            280 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
                if let Some(cb) = self.on_schedule_sta_start {
                    cb(8_000_000);
                } else {
                    self.send_wifi_sta_start_event();
                }
            }
            282 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
                self.connect_requested = true;
                self.try_start_connection();
            }
            284 => {
                if let Some(pld) = payload {
                    let mut ssid_buf = [0u8; 32];
                    let n = self.extract_ssid_from_wifi_config(pld, &mut ssid_buf);
                    if n > 0 {
                        self.target_ssid[..n as usize].copy_from_slice(&ssid_buf[..n as usize]);
                        self.target_ssid_len = n;
                    }
                }
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
                self.try_start_connection();
            }
            286 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
                self.start_scan();
            }
            288 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_disconnect_frame(uid, self.scan_result_count, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            289 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_scan_result_frame(uid, &self.scan_results, self.scan_result_count, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            353 => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = self.build_get_dhcp_dns_status_response(uid, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
            _ => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let n = build_protobuf_field(simulation_clock, uid, simulation_clock, &mut buf);
                self.send_rpc_response_packet(&buf[..n]);
            }
        }
    }

    fn build_get_wifi_mode_response(&self, uid: u32, buf: &mut [u8]) -> usize {
        build_rpc_frame(2, 515, uid, 515, &[8, 1, 16, 0], buf)
    }

    fn build_get_dhcp_dns_status_response(&self, uid: u32, buf: &mut [u8]) -> usize {
        build_rpc_frame(2, 609, uid, 609, &[8, 0], buf)
    }

    fn extract_ssid_from_wifi_config(&self, config: &[u8], out: &mut [u8]) -> u32 {
        let mut t = 0;
        while t + 2 <= config.len() {
            if 10 == config[t] {
                let ssid_len = config[t + 1] as usize;
                if ssid_len > 0 && ssid_len < 33 && t + 2 + ssid_len <= config.len() {
                    let mut valid_len = ssid_len;
                    for j in 0..ssid_len {
                        if 0 == config[t + 2 + j] {
                            valid_len = j;
                            break;
                        }
                        if config[t + 2 + j] < 32 || config[t + 2 + j] > 126 {
                            valid_len = 0;
                            break;
                        }
                    }
                    if valid_len > 0 {
                        let n = valid_len.min(out.len());
                        out[..n].copy_from_slice(&config[t + 2..t + 2 + n]);
                        return n as u32;
                    }
                }
            }
            t += 1;
        }
        0
    }

    fn add_scan_result(&mut self, result: &ScanResult) {
        let mut found = false;
        for i in 0..self.scan_result_count as usize {
            if let Some(ap) = &self.scan_results[i] {
                let mut match_all = true;
                for j in 0..6 {
                    if ap.bssid[j] != result.bssid[j] {
                        match_all = false;
                        break;
                    }
                }
                if match_all {
                    found = true;
                    break;
                }
            }
        }
        if !found && result.ssid_len > 0 {
            if (self.scan_result_count as usize) < MAX_SCAN_RESULTS {
                self.scan_results[self.scan_result_count as usize] = Some(*result);
                self.scan_result_count += 1;
            }
        }
    }

    fn start_scan(&mut self) {
        self.scan_result_count = 0;
        self.scanning = true;
        let mut buf = [0u8; 128];
        let n = build_probe_req_frame(&self.sta_mac, &[], self.wifi_seq_num, &mut buf);
        self.wifi_seq_num = self.wifi_seq_num.wrapping_add(1);
        self.transmit_wifi_frame(&buf[..n]);
        if let Some(cb) = self.on_schedule_scan_complete {
            cb(16_000_000);
        }
    }

    pub fn complete_scan(&mut self) {
        self.scanning = false;
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let n = build_connect_frame(self.scan_result_count, &mut buf);
        self.send_rpc_event_packet(&buf[..n]);
    }

    pub fn send_wifi_sta_start_event(&mut self) {
        self.sta_start_sent = true;
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let n = build_init_frame(2, &mut buf);
        self.send_rpc_event_packet(&buf[..n]);
        self.try_start_connection();
    }

    pub fn send_sta_connected_event(&mut self) {
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let n = build_scan_frame(
            &self.connected_ssid[..self.connected_ssid_len as usize],
            &self.connected_bssid,
            &mut buf,
        );
        self.send_rpc_event_packet(&buf[..n]);
    }

    fn try_start_connection(&mut self) {
        if self.connect_requested && self.sta_start_sent && self.target_ssid_len > 0 {
            self.connect_requested = false;
            let ssid_copy = self.target_ssid;
            let ssid_len = self.target_ssid_len;
            self.start_wifi_connection(&ssid_copy[..ssid_len as usize]);
        }
    }

    fn handle_priv_message(&mut self, data: &[u8]) {
        if data.len() >= 1 && 34 == data[0] {
            self.host_init_count += 1;
        }
    }

    fn queue_init_event(&mut self) {
        if self.init_event_sent { return; }
        self.init_event_sent = true;
        let payload_len: u32 = 34;
        let mut tmp_buf = [0u8; 64];
        let hdr = HciHeader {
            if_type: 5,
            if_num: 0,
            flags: 0,
            len: payload_len,
            offset: HCI_PACKET_HEADER_SIZE,
            checksum: 0,
            seq_num: self.tx_seq_num,
            throttle_cmd: 0,
            hci_pkt_type: 51,
        };
        self.tx_seq_num = self.tx_seq_num.wrapping_add(1);
        serialize_hci_header(&hdr, &mut tmp_buf);
        let mut clock_event = HCI_PACKET_HEADER_SIZE as usize;
        tmp_buf[clock_event] = 34;
        clock_event += 1;
        let simulation_clock = clock_event;
        clock_event += 1;
        let mut reg_val = 0u32;
        tmp_buf[clock_event] = 18;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        tmp_buf[clock_event] = 13;
        clock_event += 1;
        reg_val += 3;
        tmp_buf[clock_event] = 17;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        reg_val += 3;
        tmp_buf[clock_event] = 19;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        tmp_buf[clock_event] = 0;
        clock_event += 1;
        reg_val += 3;
        tmp_buf[clock_event] = 21;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        tmp_buf[clock_event] = 20;
        clock_event += 1;
        reg_val += 3;
        tmp_buf[clock_event] = 20;
        clock_event += 1;
        tmp_buf[clock_event] = 1;
        clock_event += 1;
        tmp_buf[clock_event] = 20;
        clock_event += 1;
        reg_val += 3;
        tmp_buf[clock_event] = 23;
        clock_event += 1;
        tmp_buf[clock_event] = 4;
        clock_event += 1;
        tmp_buf[clock_event] = 0;
        clock_event += 1;
        tmp_buf[clock_event] = 11;
        clock_event += 1;
        tmp_buf[clock_event] = 2;
        clock_event += 1;
        tmp_buf[clock_event] = 0;
        reg_val += 6;
        tmp_buf[simulation_clock] = reg_val as u8;
        let idx_len = 2 + reg_val;
        let hdr = HciHeader {
            if_type: 5,
            if_num: 0,
            flags: 0,
            len: idx_len,
            offset: HCI_PACKET_HEADER_SIZE,
            checksum: 0,
            seq_num: hdr.seq_num,
            throttle_cmd: 0,
            hci_pkt_type: 51,
        };
        serialize_hci_header(&hdr, &mut tmp_buf);
        let register_type = HCI_PACKET_HEADER_SIZE + idx_len;
        let checksum = compute_checksum16(&tmp_buf, register_type);
        tmp_buf[6] = checksum as u8;
        tmp_buf[7] = (checksum >> 8) as u8;
        self.sdio_slave.queue_tx_packet(&tmp_buf[..register_type as usize]);
    }

    fn send_rpc_response_packet(&mut self, data: &[u8]) {
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let n = build_serial_data_frame(&RPC_RESPONSE_ENDPOINT, data, &mut buf);
        self.send_serial_packet(&buf[..n]);
    }

    fn send_rpc_event_packet(&mut self, data: &[u8]) {
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let n = build_serial_data_frame(&RPC_EVENT_ENDPOINT, data, &mut buf);
        self.send_serial_packet(&buf[..n]);
    }

    fn send_packet(&mut self, if_type: u32, data: &[u8]) {
        let seq_num = self.tx_seq_num;
        self.tx_seq_num = self.tx_seq_num.wrapping_add(1);
        let hdr = HciHeader {
            if_type,
            if_num: 0,
            flags: 0,
            len: data.len() as u32,
            offset: HCI_PACKET_HEADER_SIZE,
            checksum: 0,
            seq_num,
            throttle_cmd: 0,
            hci_pkt_type: 0,
        };
        let idx_val = HCI_PACKET_HEADER_SIZE + data.len() as u32;
        let mut packet_buf = [0u8; MAX_PACKET_SIZE];
        serialize_hci_header(&hdr, &mut packet_buf);
        for i in 0..data.len() {
            packet_buf[HCI_PACKET_HEADER_SIZE as usize + i] = data[i];
        }
        let checksum = compute_checksum16(&packet_buf, idx_val);
        packet_buf[6] = checksum as u8;
        packet_buf[7] = (checksum >> 8) as u8;
        self.sdio_slave.queue_tx_packet(&packet_buf[..idx_val as usize]);
    }

    fn send_serial_packet(&mut self, data: &[u8]) {
        self.send_packet(3, data);
    }

    fn send_ethernet_frame_to_host(&mut self, data: &[u8]) {
        self.send_packet(1, data);
    }
}

impl MmioPeripheral for EspHostedDevice {
    fn read_u32(&mut self, _ctx: &mut CpuContext, _addr: u32) -> u32 { 0 }
    fn write_u32(&mut self, _ctx: &mut CpuContext, _addr: u32, _val: u32) {}
    fn reset(&mut self) {
        self.sdio_slave.reset();
    }
}
