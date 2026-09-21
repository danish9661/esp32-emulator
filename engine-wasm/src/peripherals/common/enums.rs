// TimerMode is defined in types.rs (used by Timer32Counter)
pub use crate::peripherals::types::TimerMode;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeripheralType {
    Gpio = 0,
    Spi = 1,
    I2c = 2,
    Uart = 3,
    Ledc = 4,
    Pcnt = 5,
    Twai = 6,
    Other = 7,
    None = 8,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinState {
    Low = 0,
    High = 1,
    Input = 2,
    PullUp = 3,
    PullDown = 4,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalDirection {
    Input = 1,
    Output = 2,
    Both = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptTrigger {
    Disable = 0,
    RisingEdge = 1,
    FallingEdge = 2,
    Edge = 3,
    LowLevel = 4,
    HighLevel = 5,
}

// I2cCommand uses consts instead of enum because STOP_NEW=2 and READ_NEW=3
// collide with READ=2 and STOP=3 (Rust enums require unique discriminants)
pub const I2C_CMD_RSTART: u32 = 0;
pub const I2C_CMD_WRITE: u32 = 1;
pub const I2C_CMD_READ: u32 = 2;
pub const I2C_CMD_STOP: u32 = 3;
pub const I2C_CMD_END: u32 = 4;
pub const I2C_CMD_RSTART_NEW: u32 = 6;
pub const I2C_CMD_STOP_NEW: u32 = 2;
pub const I2C_CMD_READ_NEW: u32 = 3;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I2cInterruptType {
    RxfifoFull = 0,
    TxfifoEmpty = 1,
    RxfifoOvf = 2,
    EndDetect = 3,
    SlaveTranComp = 4,
    ArbitrationLost = 5,
    MasterTranComp = 6,
    TransComplete = 7,
    TimeOut = 8,
    TransStart = 9,
    AckErr = 10,
    RxRecFull = 11,
    TxSendEmpty = 12,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcntRegister {
    Conf0 = 0,
    Conf1 = 1,
    Conf2 = 2,
    Cnt = 3,
    Status = 4,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmtClockSource {
    Apb = 1,
    RcFast = 2,
    Xtal = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmtChannelRegister {
    Conf0 = 0,
    Conf1 = 1,
    TxLim = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaAlgorithm {
    Sha1 = 0,
    Sha224 = 1,
    Sha256 = 2,
    Sha384 = 3,
    Sha512 = 4,
    Sha512_224 = 5,
    Sha512_256 = 6,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaPeripheralMode {
    Sha1 = 0,
    Sha256 = 1,
    Sha384 = 2,
    Sha512 = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaOperation {
    Start = 0,
    Continue = 1,
    Final = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyManagerCryptoAlgo {
    Aes = 1,
    Ecdh0 = 2,
    Ecdh1 = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPurpose {
    Invalid = 0,
    Ecdsa192 = 1,
    Ecdsa256 = 2,
    Flash256_1 = 3,
    Flash256_2 = 4,
    Flash128 = 5,
    Hmac = 6,
    Ds = 7,
    Psram256_1 = 8,
    Psram256_2 = 9,
    Psram128 = 10,
    Ecdsa384L = 11,
    Ecdsa384H = 12,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyManagerState {
    Idle = 0,
    Load = 1,
    Gain = 2,
    Busy = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XtsState {
    Idle = 0,
    Busy = 1,
    Done = 2,
    Visible = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputSignalIndex {
    LedcHsSigOut0 = 71,
    LedcLsSigOut0 = 79,
    RmtSigOut0 = 87,
}
