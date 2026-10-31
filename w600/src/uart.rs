//! 串口：W600 有 UART1、UART2，W800 有 UART1 ～ UART5。实现 `embedded-io` 的 `Read` / `Write`。
//!
//! UART0 是调试串口，由 [`println!`](crate::println) 使用。
//! W800 的 SDK 启动时会把 UART1（PB6 / PB7）用作 AT 指令口；用 [`Uart::new`] 打开 UART1 会接管它，
//! 但最好使用其他串口。

use core::time::Duration;

use embedded_io::{ErrorKind, ErrorType, Read, Write};

use crate::gpio::Pin;
use crate::sync::Semaphore;
use crate::sys::{self, TLS_UART_CHSIZE, TLS_UART_FLOW_CTRL_MODE, TLS_UART_PMODE, TLS_UART_STOPBITS};

/// UART`N` 的所有权（W600：N = 1 或 2；W800：N = 1 ～ 5）。
pub struct UART<const N: u16> {
    _private: (),
}

impl<const N: u16> UART<N> {
    pub(crate) unsafe fn steal() -> Self {
        Self { _private: () }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

/// 串口参数，默认 115200 8N1。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub baudrate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
}

impl Default for Config {
    fn default() -> Self {
        Self { baudrate: 115_200, data_bits: DataBits::Eight, parity: Parity::None, stop_bits: StopBits::One }
    }
}

impl Config {
    pub fn baudrate(baudrate: u32) -> Self {
        Self { baudrate, ..Self::default() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 初始化失败（参数不受支持或内存不足）。
    Init,
    /// SDK 返回写入失败。
    Write,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Init => f.write_str("uart init failed"),
            Error::Write => f.write_str("uart write failed"),
        }
    }
}

impl core::error::Error for Error {}

impl embedded_io::Error for Error {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

#[cfg(target_arch = "arm")]
const PORTS: usize = 3;
#[cfg(target_arch = "csky")]
const PORTS: usize = 6;

static RX_READY: [Semaphore; PORTS] = [const { Semaphore::new(0) }; PORTS];

#[cfg(target_arch = "arm")]
unsafe extern "C" fn rx_ready_1(_len: u16) -> i16 {
    RX_READY[1].release();
    0
}

#[cfg(target_arch = "arm")]
unsafe extern "C" fn rx_ready_2(_len: u16) -> i16 {
    RX_READY[2].release();
    0
}

/// W800 的回调带有注册时传入的参数，这里用它传递串口号。
#[cfg(target_arch = "csky")]
unsafe extern "C" fn rx_ready(_len: u16, port: *mut core::ffi::c_void) -> i16 {
    RX_READY[port as usize].release();
    0
}

/// 把引脚切换到 UART`port` 的复用功能。
unsafe fn config_pins(port: u16, tx: &Pin, rx: &Pin) {
    unsafe {
        match port {
            1 => {
                sys::wm_uart1_tx_config(tx.io());
                sys::wm_uart1_rx_config(rx.io());
            }
            2 => {
                sys::wm_uart2_tx_scio_config(tx.io());
                sys::wm_uart2_rx_config(rx.io());
            }
            #[cfg(target_arch = "csky")]
            3 => {
                sys::wm_uart3_tx_config(tx.io());
                sys::wm_uart3_rx_config(rx.io());
            }
            #[cfg(target_arch = "csky")]
            4 => {
                sys::wm_uart4_tx_config(tx.io());
                sys::wm_uart4_rx_config(rx.io());
            }
            #[cfg(target_arch = "csky")]
            5 => {
                sys::wm_uart5_tx_config(tx.io());
                sys::wm_uart5_rx_config(rx.io());
            }
            _ => unreachable!("invalid uart port"),
        }
    }
}

unsafe fn register_rx_callback(port: u16) {
    #[cfg(target_arch = "arm")]
    unsafe {
        let callback = if port == 1 { rx_ready_1 } else { rx_ready_2 };
        sys::tls_uart_rx_callback_register(port, Some(callback));
    }
    #[cfg(target_arch = "csky")]
    unsafe {
        sys::tls_uart_rx_callback_register(port, Some(rx_ready), port as usize as *mut core::ffi::c_void);
    }
}

unsafe fn unregister_rx_callback(port: u16) {
    #[cfg(target_arch = "arm")]
    unsafe {
        sys::tls_uart_rx_callback_register(port, None)
    };
    #[cfg(target_arch = "csky")]
    unsafe {
        sys::tls_uart_rx_callback_register(port, None, core::ptr::null_mut())
    };
}

pub struct Uart {
    port: u16,
    _tx: Pin,
    _rx: Pin,
}

impl Uart {
    pub fn new<const N: u16>(_uart: UART<N>, tx: Pin, rx: Pin, config: Config) -> Result<Self, Error> {
        let mut opts = sys::tls_uart_options_t {
            baudrate: config.baudrate,
            charlength: match config.data_bits {
                DataBits::Five => TLS_UART_CHSIZE::TLS_UART_CHSIZE_5BIT,
                DataBits::Six => TLS_UART_CHSIZE::TLS_UART_CHSIZE_6BIT,
                DataBits::Seven => TLS_UART_CHSIZE::TLS_UART_CHSIZE_7BIT,
                DataBits::Eight => TLS_UART_CHSIZE::TLS_UART_CHSIZE_8BIT,
            },
            paritytype: match config.parity {
                Parity::None => TLS_UART_PMODE::TLS_UART_PMODE_DISABLED,
                Parity::Odd => TLS_UART_PMODE::TLS_UART_PMODE_ODD,
                Parity::Even => TLS_UART_PMODE::TLS_UART_PMODE_EVEN,
            },
            flow_ctrl: TLS_UART_FLOW_CTRL_MODE::TLS_UART_FLOW_CTRL_NONE,
            stopbits: match config.stop_bits {
                StopBits::One => TLS_UART_STOPBITS::TLS_UART_ONE_STOPBITS,
                StopBits::Two => TLS_UART_STOPBITS::TLS_UART_TWO_STOPBITS,
            },
        };
        RX_READY[N as usize].init();
        unsafe {
            config_pins(N, &tx, &rx);
            if sys::tls_uart_port_init(N, &mut opts, 0) != 0 {
                return Err(Error::Init);
            }
            register_rx_callback(N);
        }
        Ok(Self { port: N, _tx: tx, _rx: rx })
    }

    /// 读取接收缓冲区中已有的数据，不等待；返回读到的字节数。
    pub fn read_available(&mut self, buf: &mut [u8]) -> usize {
        if buf.is_empty() {
            return 0;
        }
        let len = buf.len().min(u16::MAX as usize) as u16;
        let n = unsafe { sys::tls_uart_read(self.port, buf.as_mut_ptr(), len) };
        n.max(0) as usize
    }

    /// 等待最多 `timeout`，直到读到至少 1 个字节；超时返回 0。
    pub fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> usize {
        loop {
            let n = self.read_available(buf);
            if n > 0 || buf.is_empty() {
                return n;
            }
            if !RX_READY[self.port as usize].acquire_timeout(timeout) {
                return self.read_available(buf);
            }
        }
    }
}

impl Drop for Uart {
    fn drop(&mut self) {
        unsafe { unregister_rx_callback(self.port) };
    }
}

impl ErrorType for Uart {
    type Error = Error;
}

impl Read for Uart {
    /// 阻塞直到读到至少 1 个字节。
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        loop {
            let n = self.read_available(buf);
            if n > 0 || buf.is_empty() {
                return Ok(n);
            }
            RX_READY[self.port as usize].acquire();
        }
    }
}

impl Write for Uart {
    /// 阻塞直到数据发送完毕。
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = buf.len().min(u16::MAX as usize);
        // SDK 会先把数据拷贝到自己的缓冲区，这里的 *mut 只是签名需要。
        let ret = unsafe { sys::tls_uart_write(self.port, buf.as_ptr().cast_mut().cast(), len as u16) };
        if ret != 0 {
            return Err(Error::Write);
        }
        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
