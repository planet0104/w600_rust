//! SPI 主机。片选（CS）由硬件控制，每次 SDK 调用对应一次完整的片选周期。
//!
//! 因此 [`SpiDevice::transaction`] 只支持以下操作组合，其余组合返回 [`Error::Unsupported`]：
//! `[Write]`、`[Read]`、`[Write, Read]`（先发命令再读）、`[Write, Write]`（命令 + 数据）。
//! SDK 不支持全双工的 `Transfer`。

use embedded_hal::spi::{self, ErrorKind, ErrorType, Mode, Operation, Phase, Polarity, SpiDevice};

use crate::gpio::Pin;
use crate::sys;

/// SPI 外设的所有权。
pub struct SPI {
    _private: (),
}

impl SPI {
    pub(crate) unsafe fn steal() -> Self {
        Self { _private: () }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    /// 时钟频率，范围 1kHz..=APB/2（20MHz）。
    pub frequency_hz: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self { mode: spi::MODE_0, frequency_hz: 1_000_000 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 模式或频率不受支持（SDK 返回的状态码）。
    Config(i32),
    /// 传输失败（SDK 返回的状态码）。
    Transfer(i32),
    /// 硬件片选下无法实现的操作组合。
    Unsupported,
}

impl spi::Error for Error {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

pub struct Spi {
    _pins: [Pin; 4],
}

fn check(ret: i32) -> Result<(), Error> {
    if ret == 0 { Ok(()) } else { Err(Error::Transfer(ret)) }
}

impl Spi {
    pub fn new(_spi: SPI, sck: Pin, cs: Pin, miso: Pin, mosi: Pin, config: Config) -> Result<Self, Error> {
        let mode = match (config.mode.polarity, config.mode.phase) {
            (Polarity::IdleLow, Phase::CaptureOnFirstTransition) => sys::TLS_SPI_MODE_0,
            (Polarity::IdleLow, Phase::CaptureOnSecondTransition) => sys::TLS_SPI_MODE_1,
            (Polarity::IdleHigh, Phase::CaptureOnFirstTransition) => sys::TLS_SPI_MODE_2,
            (Polarity::IdleHigh, Phase::CaptureOnSecondTransition) => sys::TLS_SPI_MODE_3,
        };
        unsafe {
            sys::wm_spi_ck_config(sck.io());
            sys::wm_spi_cs_config(cs.io());
            sys::wm_spi_di_config(miso.io());
            sys::wm_spi_do_config(mosi.io());
            let ret = sys::tls_spi_setup(mode as u8, sys::TLS_SPI_CS_LOW as u8, config.frequency_hz);
            if ret != 0 {
                return Err(Error::Config(ret));
            }
        }
        Ok(Self { _pins: [sck, cs, miso, mosi] })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        check(unsafe { sys::tls_spi_write(data.as_ptr(), data.len() as u32) })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        check(unsafe { sys::tls_spi_read(buf.as_mut_ptr(), buf.len() as u32) })
    }

    /// 在同一次片选内先发送 `cmd`，再读取 `buf.len()` 个字节。
    pub fn write_then_read(&mut self, cmd: &[u8], buf: &mut [u8]) -> Result<(), Error> {
        check(unsafe { sys::tls_spi_read_with_cmd(cmd.as_ptr(), cmd.len() as u32, buf.as_mut_ptr(), buf.len() as u32) })
    }

    /// 在同一次片选内依次发送 `cmd` 和 `data`。
    pub fn write_then_write(&mut self, cmd: &[u8], data: &[u8]) -> Result<(), Error> {
        check(unsafe { sys::tls_spi_write_with_cmd(cmd.as_ptr(), cmd.len() as u32, data.as_ptr(), data.len() as u32) })
    }
}

impl ErrorType for Spi {
    type Error = Error;
}

impl SpiDevice for Spi {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Error> {
        match operations {
            [] => Ok(()),
            [Operation::Write(data)] => self.write(data),
            [Operation::Read(buf)] => self.read(buf),
            [Operation::Write(cmd), Operation::Read(buf)] => self.write_then_read(cmd, buf),
            [Operation::Write(cmd), Operation::Write(data)] => self.write_then_write(cmd, data),
            _ => Err(Error::Unsupported),
        }
    }
}
