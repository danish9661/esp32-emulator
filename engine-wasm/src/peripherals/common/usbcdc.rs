use crate::peripherals::common::twai_fifo::CircularFifoBuffer;
use crate::peripherals::types::*;

#[repr(u32)]
pub enum DataDirection {
    HostToDevice = 0,
    DeviceToHost = 1,
}

#[repr(u32)]
pub enum RequestType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[repr(u32)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}

#[repr(u32)]
pub enum StandardRequest {
    GetStatus = 0,
    ClearFeature = 1,
    Reserved1 = 2,
    SetFeature = 3,
    Reserved2 = 4,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetDeviceConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
}

#[repr(u32)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
}

pub fn build_setup_packet(
    data_direction: u32,
    req_type: u32,
    recipient: u32,
    b_request: u32,
    w_value: u32,
    w_index: u32,
    w_length: u32,
) -> [u8; 8] {
    let mut tmp_val = [0u8; 8];
    tmp_val[0] = ((data_direction << 7) | (req_type << 5) | recipient) as u8;
    tmp_val[1] = b_request as u8;
    tmp_val[2] = (255 & w_value) as u8;
    tmp_val[3] = ((w_value >> 8) & 255) as u8;
    tmp_val[4] = (255 & w_index) as u8;
    tmp_val[5] = ((w_index >> 8) & 255) as u8;
    tmp_val[6] = (255 & w_length) as u8;
    tmp_val[7] = ((w_length >> 8) & 255) as u8;
    tmp_val
}

pub fn build_set_addr_packet(cpu_val: u32) -> [u8; 8] {
    build_setup_packet(
        DataDirection::HostToDevice as u32,
        RequestType::Standard as u32,
        Recipient::Device as u32,
        StandardRequest::SetAddress as u32,
        cpu_val,
        0,
        0,
    )
}

pub fn build_get_desc_packet(cpu_val: u32, tmp_val: u32, idx_val: u32) -> [u8; 8] {
    build_setup_packet(
        DataDirection::DeviceToHost as u32,
        RequestType::Standard as u32,
        Recipient::Device as u32,
        StandardRequest::GetDescriptor as u32,
        cpu_val << 8,
        idx_val,
        tmp_val,
    )
}

pub fn build_set_config_packet(cpu_val: u32) -> [u8; 8] {
    build_setup_packet(
        DataDirection::HostToDevice as u32,
        RequestType::Standard as u32,
        Recipient::Device as u32,
        StandardRequest::SetDeviceConfiguration as u32,
        cpu_val,
        0,
        0,
    )
}

const CDC_SET_CONTROL_LINE_STATE: u32 = 34;
const CONTROL_LINE_DTR: u32 = 1;
const CONTROL_LINE_RTS: u32 = 2;
const CDC_DATA_CLASS_CODE: u32 = 10;
const ENDPOINT_TYPE_BULK: u32 = 2;
const TX_FIFO_SIZE: u32 = 512;
const CONTROL_ENDPOINT: u32 = 0;
const CONFIG_DESC_HEADER_LENGTH: u32 = 9;
const CDC_SAB_RING_SIZE: u32 = 512;
const CDC_SAB_TOTAL_SLOTS: u32 = CDC_SAB_RING_SIZE + 2;

pub fn find_bulk_endpoints(cpu_val: &[u8]) -> (i32, i32) {
    let mut tmp_val = 0u32;
    let mut idx_val = false;
    let mut in_endpoint = -1i32;
    let mut out_endpoint = -1i32;
    while (tmp_val as usize) < cpu_val.len() {
        let simulation_clock = cpu_val[tmp_val as usize] as u32;
        if simulation_clock < 2 || (cpu_val.len() as u32) < tmp_val + simulation_clock {
            break;
        }
        let reg_val = cpu_val[(tmp_val + 1) as usize] as u32;
        if reg_val == DescriptorType::Interface as u32 && 9 == simulation_clock {
            let clock_event = cpu_val[(tmp_val + 4) as usize] as u32;
            let simulation_clock_inner = cpu_val[(tmp_val + 5) as usize] as u32;
            idx_val = 2 == clock_event && simulation_clock_inner == CDC_DATA_CLASS_CODE;
        }
        if idx_val && reg_val == DescriptorType::Endpoint as u32 && 7 == simulation_clock {
            let idx_val_inner = cpu_val[(tmp_val + 2) as usize] as u32;
            if (3 & cpu_val[(tmp_val + 3) as usize] as u32) == ENDPOINT_TYPE_BULK {
                if 0 != (128 & idx_val_inner) {
                    in_endpoint = (15 & idx_val_inner) as i32;
                } else {
                    out_endpoint = (15 & idx_val_inner) as i32;
                }
            }
        }
        tmp_val += cpu_val[tmp_val as usize] as u32;
    }
    (in_endpoint, out_endpoint)
}

pub struct UsbCdcAcm {
    pub usb: *mut u8,
    pub tx_fifo: CircularFifoBuffer,
    pub initialized: bool,
    pub descriptors_size: u32,
    pub descriptors: [u8; 512],
    pub descriptors_len: u32,
    pub out_endpoint: i32,
    pub in_endpoint: i32,
    pub _tx_sab: *mut i32,
    pub _tx_view: *mut i32,
    pub _tx_head: u32,
    pub _rx_sab: *mut i32,
    pub _rx_view: *mut i32,
    pub _rx_tail: u32,
    pub on_reset_received: Option<fn(&mut UsbCdcAcm)>,
    pub on_endpoint_write: Option<fn(&mut UsbCdcAcm, u32, &[u8])>,
    pub on_endpoint_read: Option<fn(&mut UsbCdcAcm, u32, u32)>,
    pub on_device_connected: Option<fn(&mut UsbCdcAcm)>,
    pub on_serial_data: Option<fn(&mut UsbCdcAcm, &[u8])>,
}

impl UsbCdcAcm {
    pub fn new(cpu_val: *mut u8) -> Self {
        UsbCdcAcm {
            usb: cpu_val,
            tx_fifo: CircularFifoBuffer::new(),
            initialized: false,
            descriptors_size: 0,
            descriptors: [0u8; 512],
            descriptors_len: 0,
            out_endpoint: -1,
            in_endpoint: -1,
            _tx_sab: core::ptr::null_mut(),
            _tx_view: core::ptr::null_mut(),
            _tx_head: 0,
            _rx_sab: core::ptr::null_mut(),
            _rx_view: core::ptr::null_mut(),
            _rx_tail: 0,
            on_reset_received: Some(Self::on_reset_received_impl),
            on_endpoint_write: Some(Self::on_endpoint_write_impl),
            on_endpoint_read: Some(Self::on_endpoint_read_impl),
            on_device_connected: None,
            on_serial_data: None,
        }
    }

    pub fn cdc_set_control_line_state(
        &mut self,
        cpu_val: u32,
        tmp_val: u32,
    ) {
        self.send_setup_packet(build_setup_packet(
            DataDirection::HostToDevice as u32,
            RequestType::Class as u32,
            Recipient::Device as u32,
            CDC_SET_CONTROL_LINE_STATE,
            cpu_val,
            tmp_val,
            0,
        ));
        self.initialized = true;
    }

    pub fn send_serial_byte(&mut self, cpu_val: u8) {
        self.tx_fifo.push_byte(cpu_val);
        if !self._tx_view.is_null() {
            self.push_tx_byte(cpu_val);
        }
    }

    pub fn attach_serial_sab(&mut self, tx_sab: *mut i32, rx_sab: *mut i32) {
        if !tx_sab.is_null() {
            self._tx_sab = tx_sab;
            self._tx_view = tx_sab;
            unsafe {
                *self._tx_view.add(0) = 2;
                *self._tx_view.add(1) = 2;
            }
            self._tx_head = 2;
        }
        if !rx_sab.is_null() {
            self._rx_sab = rx_sab;
            self._rx_view = rx_sab;
            let loaded = unsafe { core::ptr::read_volatile(self._rx_view.add(1)) };
            self._rx_tail = if loaded != 0 { loaded as u32 } else { 2 };
        }
    }

    pub fn detach_serial_sab(&mut self) {
        self._tx_sab = core::ptr::null_mut();
        self._tx_view = core::ptr::null_mut();
        self._rx_sab = core::ptr::null_mut();
        self._rx_view = core::ptr::null_mut();
    }

    pub fn push_tx_byte(&mut self, cpu_val: u8) {
        if self._tx_view.is_null() {
            return;
        }
        let mut next = self._tx_head + 1;
        if next >= CDC_SAB_TOTAL_SLOTS {
            next = 2;
        }
        if next == unsafe { core::ptr::read_volatile(self._tx_view.add(1)) as u32 } {
            return;
        }
        unsafe {
            *self._tx_view.add(self._tx_head as usize) = cpu_val as i32;
            core::ptr::write_volatile(self._tx_view.add(0), next as i32);
        }
        self._tx_head = next;
    }

    pub fn pull_rx_bytes(&mut self, cpu_val: u32, buf: &mut [u8]) -> u32 {
        if self._rx_view.is_null() {
            return 0;
        }
        let reg_val = unsafe { core::ptr::read_volatile(self._rx_view.add(0)) as u32 };
        let mut arg_val = self._rx_tail;
        let mut count = 0u32;
        while arg_val != reg_val && count < cpu_val {
            buf[count as usize] = unsafe { *self._rx_view.add(arg_val as usize) } as u8;
            arg_val += 1;
            if arg_val >= CDC_SAB_TOTAL_SLOTS {
                arg_val = 2;
            }
            count += 1;
        }
        if count != 0 {
            self._rx_tail = arg_val;
            unsafe { core::ptr::write_volatile(self._rx_view.add(1), arg_val as i32); }
        }
        count
    }

    fn send_setup_packet(&mut self, _packet: [u8; 8]) {
    }

    fn endpoint_read_done(&mut self, _endpoint: i32, _data: &[u8]) {
    }

    fn on_reset_received_impl(this: &mut UsbCdcAcm) {
        this.send_setup_packet(build_set_addr_packet(1));
    }

    fn on_endpoint_write_impl(this: &mut UsbCdcAcm, cpu_val: u32, tmp_val: &[u8]) {
        if cpu_val == CONTROL_ENDPOINT && tmp_val.len() == 0 {
            if this.descriptors_size == 0 {
                this.send_setup_packet(build_get_desc_packet(
                    DescriptorType::Configuration as u32,
                    CONFIG_DESC_HEADER_LENGTH,
                    0,
                ));
            } else if !this.initialized {
                this.cdc_set_control_line_state(CONTROL_LINE_DTR | CONTROL_LINE_RTS, 0);
                if let Some(cb) = this.on_device_connected {
                    cb(this);
                }
            }
        }
        if cpu_val == CONTROL_ENDPOINT && tmp_val.len() > 1 {
            if tmp_val.len() as u32 == CONFIG_DESC_HEADER_LENGTH
                && tmp_val[1] as u32 == DescriptorType::Configuration as u32
                && this.descriptors_size == 0
            {
                this.descriptors_size = (tmp_val[3] as u32) << 8 | tmp_val[2] as u32;
                this.send_setup_packet(build_get_desc_packet(
                    DescriptorType::Configuration as u32,
                    this.descriptors_size,
                    0,
                ));
            } else if this.descriptors_size != 0
                && (this.descriptors_len as u32) < this.descriptors_size
            {
                let avail = this.descriptors_size - this.descriptors_len;
                let copy_len = core::cmp::min(tmp_val.len() as u32, avail) as usize;
                for i in 0..copy_len {
                    this.descriptors[this.descriptors_len as usize + i] = tmp_val[i];
                }
                this.descriptors_len += copy_len as u32;
            }
        }
        if this.descriptors_size != 0 && this.descriptors_len as u32 == this.descriptors_size {
            let endpoints = find_bulk_endpoints(&this.descriptors[..this.descriptors_len as usize]);
            this.in_endpoint = endpoints.0;
            this.out_endpoint = endpoints.1;
            this.send_setup_packet(build_set_config_packet(1));
        }
        if cpu_val as i32 == this.in_endpoint {
            if let Some(cb) = this.on_serial_data {
                cb(this, tmp_val);
            }
        }
    }

    fn on_endpoint_read_impl(this: &mut UsbCdcAcm, cpu_val: u32, tmp_val: u32) {
        if cpu_val as i32 == this.out_endpoint {
            let mut data = [0u8; 1024];
            let len: u32;
            if !this._rx_view.is_null() {
                len = this.pull_rx_bytes(tmp_val, &mut data);
            } else {
                let count = core::cmp::min(tmp_val, this.tx_fifo.item_count());
                for i in 0..count {
                    data[i as usize] = this.tx_fifo.pull() as u8;
                }
                len = count;
            }
            this.endpoint_read_done(this.out_endpoint, &data[..len as usize]);
        }
    }
}
