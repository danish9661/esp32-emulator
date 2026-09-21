use crate::peripherals::types::*;

const DEVICE_DESCRIPTOR_RAW: [u8; 18] = [
    18, 1, 0, 2, 0, 0, 0, 8, 109, 4, 1, 197, 16, 1, 1, 2, 0, 1,
];

const CONFIG_DESCRIPTOR_RAW: [u8; 34] = [
    9, 2, 34, 0, 1, 1, 0, 160, 50, 9, 4, 0, 0, 1, 3, 1, 1, 0,
    9, 33, 17, 1, 0, 1, 34, 63, 0, 7, 5, 129, 3, 8, 0, 10,
];

const HID_REPORT_DESCRIPTOR: [u8; 63] = [
    5, 1, 9, 6, 161, 1, 5, 7, 25, 224, 41, 231, 21, 0, 37, 1,
    117, 1, 149, 8, 129, 2, 149, 1, 117, 8, 129, 1, 149, 5, 117, 1,
    5, 8, 25, 1, 41, 5, 145, 2, 149, 1, 117, 3, 145, 1, 149, 6,
    117, 8, 21, 0, 37, 255, 5, 7, 25, 0, 41, 255, 129, 0, 192,
];

const STRING_DESCRIPTOR_ZERO: [u8; 4] = [4, 3, 9, 4];

const MANUFACTURER_STRING_DESCRIPTOR: [u8; 12] = [
    12, 3, 69, 0, 83, 0, 80, 0, 51, 0, 50, 0,
];

const PRODUCT_STRING_DESCRIPTOR: [u8; 34] = [
    34, 3,
    86, 0, 105, 0, 114, 0, 116, 0, 117, 0, 97, 0, 108, 0, 32, 0,
    75, 0, 101, 0, 121, 0, 98, 0, 111, 0, 97, 0, 114, 0, 100, 0,
];

const HID_DESCRIPTOR_SUBARRAY: [u8; 9] = [9, 33, 17, 1, 0, 1, 34, 63, 0];

const MODIFIER_LEFT_SHIFT: u32 = 2;

const HID_KEY_A: u32 = 4;
const HID_KEY_1: u32 = 30;
const HID_KEY_2: u32 = 31;
const HID_KEY_3: u32 = 32;
const HID_KEY_4: u32 = 33;
const HID_KEY_5: u32 = 34;
const HID_KEY_6: u32 = 35;
const HID_KEY_7: u32 = 36;
const HID_KEY_8: u32 = 37;
const HID_KEY_9: u32 = 38;
const HID_KEY_0: u32 = 39;
const HID_KEY_ENTER: u32 = 40;
const HID_KEY_TAB: u32 = 43;
const HID_KEY_SPACE: u32 = 44;
const HID_KEY_MINUS: u32 = 45;
const HID_KEY_EQUAL: u32 = 46;
const HID_KEY_LEFT_BRACKET: u32 = 47;
const HID_KEY_RIGHT_BRACKET: u32 = 48;
const HID_KEY_BACKSLASH: u32 = 49;
const HID_KEY_SEMICOLON: u32 = 51;
const HID_KEY_QUOTE: u32 = 52;
const HID_KEY_GRAVE: u32 = 53;
const HID_KEY_COMMA: u32 = 54;
const HID_KEY_PERIOD: u32 = 55;
const HID_KEY_SLASH: u32 = 56;

const KBD_SAB_SLOTS: u32 = 9;
const MAX_PENDING_REPORTS: usize = 64;
const REPORT_SIZE: usize = 8;

struct KeyCodeResult {
    key_code: u32,
    shift: u32,
}

fn char_to_key_code(c: char) -> KeyCodeResult {
    let code = c as u32;
    if (97..=122).contains(&code) {
        return KeyCodeResult { key_code: code - 97 + HID_KEY_A, shift: false as u32 };
    }
    if (65..=90).contains(&code) {
        return KeyCodeResult { key_code: code - 65 + HID_KEY_A, shift: true as u32 };
    }
    if (49..=57).contains(&code) {
        return KeyCodeResult { key_code: code - 49 + HID_KEY_1, shift: false as u32 };
    }
    if code == 48 {
        return KeyCodeResult { key_code: HID_KEY_0, shift: false as u32 };
    }
    match c {
        ' ' => KeyCodeResult { key_code: HID_KEY_SPACE, shift: false as u32 },
        '\n' | '\r' => KeyCodeResult { key_code: HID_KEY_ENTER, shift: false as u32 },
        '\t' => KeyCodeResult { key_code: HID_KEY_TAB, shift: false as u32 },
        '-' => KeyCodeResult { key_code: HID_KEY_MINUS, shift: false as u32 },
        '_' => KeyCodeResult { key_code: HID_KEY_MINUS, shift: true as u32 },
        '=' => KeyCodeResult { key_code: HID_KEY_EQUAL, shift: false as u32 },
        '+' => KeyCodeResult { key_code: HID_KEY_EQUAL, shift: true as u32 },
        '[' => KeyCodeResult { key_code: HID_KEY_LEFT_BRACKET, shift: false as u32 },
        '{' => KeyCodeResult { key_code: HID_KEY_LEFT_BRACKET, shift: true as u32 },
        ']' => KeyCodeResult { key_code: HID_KEY_RIGHT_BRACKET, shift: false as u32 },
        '}' => KeyCodeResult { key_code: HID_KEY_RIGHT_BRACKET, shift: true as u32 },
        '\\' => KeyCodeResult { key_code: HID_KEY_BACKSLASH, shift: false as u32 },
        '|' => KeyCodeResult { key_code: HID_KEY_BACKSLASH, shift: true as u32 },
        ';' => KeyCodeResult { key_code: HID_KEY_SEMICOLON, shift: false as u32 },
        ':' => KeyCodeResult { key_code: HID_KEY_SEMICOLON, shift: true as u32 },
        '\'' => KeyCodeResult { key_code: HID_KEY_QUOTE, shift: false as u32 },
        '"' => KeyCodeResult { key_code: HID_KEY_QUOTE, shift: true as u32 },
        '`' => KeyCodeResult { key_code: HID_KEY_GRAVE, shift: false as u32 },
        '~' => KeyCodeResult { key_code: HID_KEY_GRAVE, shift: true as u32 },
        ',' => KeyCodeResult { key_code: HID_KEY_COMMA, shift: false as u32 },
        '<' => KeyCodeResult { key_code: HID_KEY_COMMA, shift: true as u32 },
        '.' => KeyCodeResult { key_code: HID_KEY_PERIOD, shift: false as u32 },
        '>' => KeyCodeResult { key_code: HID_KEY_PERIOD, shift: true as u32 },
        '/' => KeyCodeResult { key_code: HID_KEY_SLASH, shift: false as u32 },
        '?' => KeyCodeResult { key_code: HID_KEY_SLASH, shift: true as u32 },
        '!' => KeyCodeResult { key_code: HID_KEY_1, shift: true as u32 },
        '@' => KeyCodeResult { key_code: HID_KEY_2, shift: true as u32 },
        '#' => KeyCodeResult { key_code: HID_KEY_3, shift: true as u32 },
        '$' => KeyCodeResult { key_code: HID_KEY_4, shift: true as u32 },
        '%' => KeyCodeResult { key_code: HID_KEY_5, shift: true as u32 },
        '^' => KeyCodeResult { key_code: HID_KEY_6, shift: true as u32 },
        '&' => KeyCodeResult { key_code: HID_KEY_7, shift: true as u32 },
        '*' => KeyCodeResult { key_code: HID_KEY_8, shift: true as u32 },
        '(' => KeyCodeResult { key_code: HID_KEY_9, shift: true as u32 },
        ')' => KeyCodeResult { key_code: HID_KEY_0, shift: true as u32 },
        _ => KeyCodeResult { key_code: 0, shift: false as u32 },
    }
}

pub struct UsbKeyboard {
    pub address: u32,
    pub configured: u32,
    pub speed: u32,
    pub protocol: u32,
    pub idle_rate: u32,
    pub modifiers: u32,
    pub keys: [u32; 6],
    pub pending_reports: [[u8; REPORT_SIZE]; MAX_PENDING_REPORTS],
    pub pending_count: u32,
    pub _sab: *mut u8,
    pub _sab_view: *mut i32,
    pub response_buf: [u8; REPORT_SIZE],
}

impl UsbKeyboard {
    pub fn new() -> Self {
        UsbKeyboard {
            address: 0,
            configured: 0,
            speed: 1,
            protocol: 0,
            idle_rate: 0,
            modifiers: 0,
            keys: [0, 0, 0, 0, 0, 0],
            pending_reports: [[0u8; REPORT_SIZE]; MAX_PENDING_REPORTS],
            pending_count: 0,
            _sab: core::ptr::null_mut(),
            _sab_view: core::ptr::null_mut(),
            response_buf: [0u8; REPORT_SIZE],
        }
    }

    pub fn device_descriptor(&self) -> &'static [u8] {
        &DEVICE_DESCRIPTOR_RAW
    }

    pub fn config_descriptor(&self) -> &'static [u8] {
        &CONFIG_DESCRIPTOR_RAW
    }

    pub fn press_key(&mut self, key_code: u32, modifier: u32) {
        self.modifiers |= modifier;
        for i in 0..6 {
            if self.keys[i] == 0 {
                self.keys[i] = key_code;
                break;
            }
        }
        self.update_report();
    }

    pub fn release_key(&mut self, key_code: u32) {
        for t in 0..6 {
            if self.keys[t] == key_code {
                self.keys[t] = 0;
            }
        }
        self.modifiers = 0;
        self.update_report();
    }

    pub fn release_all_keys(&mut self) {
        self.modifiers = 0;
        for i in 0..6 {
            self.keys[i] = 0;
        }
        self.update_report();
    }

    pub fn type_char(&mut self, c: char) {
        let result = char_to_key_code(c);
        let tmp_val = result.key_code;
        let idx_val = result.shift;
        if tmp_val != 0 {
            self.press_key(tmp_val, if idx_val != 0 { MODIFIER_LEFT_SHIFT } else { 0 });
            self.release_key(tmp_val);
        }
    }

    pub fn update_report(&mut self) {
        if self.pending_count < MAX_PENDING_REPORTS as u32 {
            let mut report = [0u8; REPORT_SIZE];
            report[0] = self.modifiers as u8;
            report[1] = 0;
            report[2] = self.keys[0] as u8;
            report[3] = self.keys[1] as u8;
            report[4] = self.keys[2] as u8;
            report[5] = self.keys[3] as u8;
            report[6] = self.keys[4] as u8;
            report[7] = self.keys[5] as u8;
            self.pending_reports[self.pending_count as usize] = report;
            self.pending_count += 1;
        }
        if !self._sab_view.is_null() {
            unsafe {
                *self._sab_view.add(0) = 1;
                *self._sab_view.add(1) = self.modifiers as i32;
                for t in 0..6 {
                    *self._sab_view.add(2 + t) = self.keys[t] as i32;
                }
            }
        }
    }

    pub fn attach_keyboard_sab(&mut self, sab: *mut u8) {
        self._sab = sab;
        self._sab_view = sab as *mut i32;
        if !self._sab_view.is_null() {
            unsafe { *self._sab_view.add(0) = 0; }
        }
    }

    pub fn detach_keyboard_sab(&mut self) {
        self._sab = core::ptr::null_mut();
        self._sab_view = core::ptr::null_mut();
    }

    pub fn handle_setup_packet(&mut self, data: &[u8]) -> Option<&[u8]> {
        let tmp_val = data[0];
        let idx_val = data[1];
        let clock_event = (data[2] as u32) | ((data[3] as u32) << 8);
        let _ = data[4]; let _ = data[5];
        let simulation_clock = (data[6] as u32) | ((data[7] as u32) << 8);
        let reg_val = (tmp_val >> 5) & 3;
        let arg_val = 31 & tmp_val;
        if reg_val == 0 {
            match idx_val {
                0 => Some(&[0u8, 0]),
                5 => {
                    self.address = clock_event;
                    Some(&[])
                }
                6 => self.get_descriptor(clock_event >> 8, 255 & clock_event, simulation_clock),
                8 => {
                    self.response_buf[0] = if self.configured != 0 { 1 } else { 0 };
                    Some(&self.response_buf[..1])
                }
                9 => {
                    self.configured = if clock_event == 1 { 1 } else { 0 };
                    Some(&[])
                }
                _ => None,
            }
        } else if reg_val == 1 && arg_val == 1 {
            match idx_val {
                1 => Some(self.get_current_report()),
                2 => {
                    self.response_buf[0] = self.idle_rate as u8;
                    Some(&self.response_buf[..1])
                }
                3 => {
                    self.response_buf[0] = self.protocol as u8;
                    Some(&self.response_buf[..1])
                }
                9 => Some(&[]),
                10 => {
                    self.idle_rate = clock_event >> 8;
                    Some(&[])
                }
                11 => {
                    self.protocol = clock_event;
                    Some(&[])
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn handle_control_data(&mut self, _data: &[u8]) {}

    pub fn handle_out_data(&mut self, _ep: u32, _data: &[u8]) {}

    pub fn get_in_data(&mut self, ep: u32, max_len: u32) -> Option<&[u8]> {
        if ep == 1 && self.pending_count > 0 {
            let report = self.pending_reports[0];
            for i in 1..self.pending_count as usize {
                self.pending_reports[i - 1] = self.pending_reports[i];
            }
            self.pending_count -= 1;
            let len = core::cmp::min(max_len as usize, REPORT_SIZE);
            self.response_buf[..len].copy_from_slice(&report[..len]);
            Some(&self.response_buf[..len])
        } else {
            None
        }
    }

    pub fn get_descriptor(&self, desc_type: u32, desc_index: u32, w_length: u32) -> Option<&[u8]> {
        let clock_event: Option<&[u8]> = match desc_type {
            1 => Some(&DEVICE_DESCRIPTOR_RAW),
            2 => Some(&CONFIG_DESCRIPTOR_RAW),
            3 => match desc_index {
                0 => Some(&STRING_DESCRIPTOR_ZERO),
                1 => Some(&MANUFACTURER_STRING_DESCRIPTOR),
                2 => Some(&PRODUCT_STRING_DESCRIPTOR),
                _ => None,
            },
            33 => Some(&HID_DESCRIPTOR_SUBARRAY),
            34 => Some(&HID_REPORT_DESCRIPTOR),
            _ => None,
        };
        match clock_event {
            Some(d) => {
                let len = core::cmp::min(w_length as usize, d.len());
                Some(&d[..len])
            }
            None => None,
        }
    }

    pub fn get_current_report(&mut self) -> &[u8] {
        self.response_buf[0] = self.modifiers as u8;
        self.response_buf[1] = 0;
        self.response_buf[2] = self.keys[0] as u8;
        self.response_buf[3] = self.keys[1] as u8;
        self.response_buf[4] = self.keys[2] as u8;
        self.response_buf[5] = self.keys[3] as u8;
        self.response_buf[6] = self.keys[4] as u8;
        self.response_buf[7] = self.keys[5] as u8;
        &self.response_buf
    }
}

impl MmioPeripheral for UsbKeyboard {
    fn read_u32(&mut self, _ctx: &mut CpuContext, _addr: u32) -> u32 { 0 }
    fn write_u32(&mut self, _ctx: &mut CpuContext, _addr: u32, _val: u32) {}
    fn reset(&mut self) {
        self.address = 0;
        self.configured = 0;
        self.speed = 1;
        self.protocol = 0;
        self.idle_rate = 0;
        self.modifiers = 0;
        self.keys = [0, 0, 0, 0, 0, 0];
        self.pending_count = 0;
        self.response_buf = [0u8; REPORT_SIZE];
    }
}