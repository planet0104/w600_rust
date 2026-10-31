//! I2C 主机（7 位地址）。实现 `embedded-hal` 的 `I2c`。

use embedded_hal::i2c::{self, ErrorKind, ErrorType, NoAcknowledgeSource, Operation};

use crate::gpio::Pin;
use crate::sys;

/// I2C 外设的所有权。
pub struct I2C {
    _private: (),
}

impl I2C {
    pub(crate) unsafe fn steal() -> Self {
        Self { _private: () }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NoAcknowledge(NoAcknowledgeSource),
}

impl i2c::Error for Error {
    fn kind(&self) -> ErrorKind {
        match *self {
            Error::NoAcknowledge(source) => ErrorKind::NoAcknowledge(source),
        }
    }
}

pub struct I2c {
    _scl: Pin,
    _sda: Pin,
}

impl I2c {
    /// `freq_hz` 会被 SDK 限制在支持的范围内（通常 100k / 400k）。
    pub fn new(_i2c: I2C, scl: Pin, sda: Pin, freq_hz: u32) -> Self {
        unsafe {
            sys::wm_i2c_scl_config(scl.io());
            sys::wm_i2c_sda_config(sda.io());
            sys::tls_i2c_init(freq_hz);
        }
        Self { _scl: scl, _sda: sda }
    }
}

fn wait_ack(source: NoAcknowledgeSource) -> Result<(), Error> {
    // 收到 NACK 时 SDK 已经发送了停止位。
    if unsafe { sys::tls_i2c_wait_ack() } == 0 { Ok(()) } else { Err(Error::NoAcknowledge(source)) }
}

impl ErrorType for I2c {
    type Error = Error;
}

impl i2c::I2c for I2c {
    fn transaction(&mut self, address: u8, operations: &mut [Operation<'_>]) -> Result<(), Error> {
        let count = operations.len();
        let mut direction: Option<bool> = None;
        let mut stopped = false;
        for i in 0..count {
            let is_read = matches!(operations[i], Operation::Read(_));
            // 方向改变（或第一次）时发送起始位/重复起始位和地址；同方向的相邻操作直接拼接。
            if direction != Some(is_read) {
                unsafe { sys::tls_i2c_write_byte((address << 1) | is_read as u8, 1) };
                wait_ack(NoAcknowledgeSource::Address)?;
                direction = Some(is_read);
            }
            let read_continues = matches!(operations.get(i + 1), Some(Operation::Read(_)));
            let is_last = i + 1 == count;
            match &mut operations[i] {
                Operation::Write(bytes) => {
                    for &b in bytes.iter() {
                        unsafe { sys::tls_i2c_write_byte(b, 0) };
                        wait_ack(NoAcknowledgeSource::Data)?;
                    }
                }
                Operation::Read(buf) => {
                    let len = buf.len();
                    for (j, b) in buf.iter_mut().enumerate() {
                        // 一段连续读取的最后一个字节回 NACK；整个事务的最后一个字节同时发送停止位。
                        let last_byte = j + 1 == len && !read_continues;
                        let stop = last_byte && is_last;
                        *b = unsafe { sys::tls_i2c_read_byte(!last_byte as u8, stop as u8) };
                        stopped |= stop;
                    }
                }
            }
        }
        if count > 0 && !stopped {
            unsafe { sys::tls_i2c_stop() };
        }
        Ok(())
    }
}
