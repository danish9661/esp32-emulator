use crate::peripherals::types::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorCoding {
    RGB565 = 0,
    RGB666 = 1,
    RGB888 = 2,
}

pub struct MipiDsiDevice {
    pub base_addr: u32,
    pub frame_buffer: u32,
    pub frame_buffer_bytes: u32,
    pub color_coding: ColorCoding,
    pub width: u32,
    pub height: u32,
    pub sab: *mut u32,
    pub sab_view: *mut u32,
}

impl MipiDsiDevice {
    pub fn new(base_addr: u32) -> Self {
        MipiDsiDevice {
            base_addr,
            frame_buffer: 0,
            frame_buffer_bytes: 0,
            color_coding: ColorCoding::RGB565,
            width: 0,
            height: 0,
            sab: core::ptr::null_mut(),
            sab_view: core::ptr::null_mut(),
        }
    }

    pub fn bytes_per_pixel(&self) -> u32 {
        match self.color_coding {
            ColorCoding::RGB565 => 2,
            ColorCoding::RGB666 | ColorCoding::RGB888 => 3,
            _ => 2,
        }
    }

    pub fn attach_display_sab(&mut self, sab: *mut u32) {
        self.sab = sab;
        self.sab_view = sab;
        unsafe {
            *self.sab_view.add(1) = self.width;
            *self.sab_view.add(2) = self.height;
            *self.sab_view.add(3) = self.bytes_per_pixel();
            *self.sab_view.add(4) = self.frame_buffer;
            *self.sab_view.add(5) = self.frame_buffer_bytes;
        }
    }

    pub fn detach_display_sab(&mut self) {
        self.sab = core::ptr::null_mut();
        self.sab_view = core::ptr::null_mut();
    }

    pub fn invalidate(&mut self) {
        if !self.sab_view.is_null() {
            unsafe { *self.sab_view = 1; }
        }
    }
}

impl MmioPeripheral for MipiDsiDevice {
    fn read_u32(&mut self, _ctx: &mut CpuContext, _addr: u32) -> u32 {
        0
    }

    fn write_u32(&mut self, _ctx: &mut CpuContext, _addr: u32, _val: u32) {}

    fn reset(&mut self) {
        self.frame_buffer = 0;
        self.frame_buffer_bytes = 0;
        self.color_coding = ColorCoding::RGB565;
        self.width = 0;
        self.height = 0;
        self.sab = core::ptr::null_mut();
        self.sab_view = core::ptr::null_mut();
    }
}
