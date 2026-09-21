#![no_std]

#[cfg(feature = "xtensa")]
mod xtensa;
mod crypto;
mod native_mmio;
pub mod peripherals;

extern "C" {
    fn js_log_str(ptr: u32, len: u32);
}

struct PanicBuf([u8; 512]);

use core::fmt::Write as _;

impl core::fmt::Write for PanicBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut i = 0;
        for b in s.bytes() {
            if i >= self.0.len() {
                return Ok(());
            }
            self.0[i] = b;
            i += 1;
        }
        Ok(())
    }
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    let mut buf = PanicBuf([0u8; 512]);
    let _ = core::write!(&mut buf, "[WASM PANIC] {}", info);
    unsafe { js_log_str(&buf.0 as *const u8 as u32, 512) };
    loop {}
}
