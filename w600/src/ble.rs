//! BLE 从机（仅 W800）：广播设备名，接受手机等主机连接，并接收主机写入的数据。
//!
//! 基于 SDK 自带的 NimBLE 协议栈和其中的 GATT 服务（`src/app/bleapp/wm_ble_server_api_demo.c`）：
//!
//! | UUID | 属性 | 用途 |
//! | --- | --- | --- |
//! | `0xFFF0` | 服务 | 广播中也包含此 UUID |
//! | `0xFFF2` | Write | 主机写入的数据以 [`Event::Received`] 交给应用 |
//! | `0xFFF3` | Read | SDK 的测试数据 |
//! | `0xFFF1` / `0xFFF4` | Indicate / Notify | SDK 保留 |
//!
//! ```no_run
//! let server = w600::ble::Server::start("W800-Rust").unwrap();
//! loop {
//!     if let w600::ble::Event::Received(data) = server.recv() {
//!         w600::println!("{:?}", data);
//!     }
//! }
//! ```

use alloc::vec::Vec;
use core::time::Duration;

use crate::sync::Channel;
use crate::sys::{self, tls_uart_msg_out_t};

/// NimBLE 的 `BLE_HS_EALREADY`：协议栈已经启动。
const BLE_HS_EALREADY: i32 = 2;

/// 广播包最多 31 字节，扣除标志（3）、服务 UUID（4）和名字字段头（2）后剩余 22 字节。
pub const MAX_NAME_LEN: usize = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 设备名为空、超过 [`MAX_NAME_LEN`] 字节或包含 NUL。
    InvalidName,
    /// 服务已经启动。
    AlreadyStarted,
    /// NimBLE / SDK 返回的错误码。
    Stack(i32),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InvalidName => f.write_str("invalid ble device name"),
            Error::AlreadyStarted => f.write_str("ble server already started"),
            Error::Stack(rc) => write!(f, "ble stack error {rc}"),
        }
    }
}

impl core::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// 开始广播，可以被扫描和连接。
    Advertising,
    /// 主机已连接。
    Connected,
    /// 主机断开（HCI 断开原因码），之后 SDK 会自动重新广播。
    Disconnected { reason: u8 },
    /// 主机向 `0xFFF2` 特征写入的数据。
    Received(Vec<u8>),
}

static EVENTS: Channel<Event> = Channel::new(16);

/// 在 NimBLE 的主机任务中调用。没人读取时队列会满，丢掉新的事件。
unsafe extern "C" fn on_event(ty: tls_uart_msg_out_t, payload: *mut u8, length: i32) {
    let event = match ty {
        tls_uart_msg_out_t::UART_OUTPUT_DATA if !payload.is_null() && length > 0 => {
            Event::Received(unsafe { core::slice::from_raw_parts(payload, length as usize) }.to_vec())
        }
        tls_uart_msg_out_t::UART_OUTPUT_CMD_ADVERTISING => Event::Advertising,
        tls_uart_msg_out_t::UART_OUTPUT_CMD_CONNECTED => Event::Connected,
        // SDK 把原因码放在 payload[0]，但 length 传的是 0。
        tls_uart_msg_out_t::UART_OUTPUT_CMD_DISCONNECTED => {
            Event::Disconnected { reason: if payload.is_null() { 0 } else { unsafe { *payload } } }
        }
        _ => return,
    };
    let _ = EVENTS.send(event);
}

fn check(rc: i32) -> Result<(), Error> {
    if rc == 0 { Ok(()) } else { Err(Error::Stack(rc)) }
}

/// GATT 服务端。同一时间只能有一个，丢弃时停止广播并断开连接（协议栈保持运行）。
pub struct Server {
    _private: (),
}

impl Server {
    /// 启动蓝牙协议栈（首次调用时，约需数百毫秒），以 `name` 为设备名开始广播。
    pub fn start(name: &str) -> Result<Server, Error> {
        if name.is_empty() || name.len() > MAX_NAME_LEN || name.contains('\0') {
            return Err(Error::InvalidName);
        }
        let mut c_name = [0u8; MAX_NAME_LEN + 1];
        c_name[..name.len()].copy_from_slice(name.as_bytes());

        EVENTS.init();
        unsafe {
            let rc = sys::tls_bt_init(0xFF);
            if rc != BLE_HS_EALREADY {
                check(rc)?;
            }
            check(sys::ble_svc_gap_device_name_set(c_name.as_ptr().cast()))?;
            match sys::tls_ble_server_demo_api_init(Some(on_event), None) {
                BLE_HS_EALREADY => return Err(Error::AlreadyStarted),
                rc => check(rc)?,
            }
        }
        Ok(Server { _private: () })
    }

    /// 等待下一个事件。
    pub fn recv(&self) -> Event {
        EVENTS.recv()
    }

    /// 在 `timeout` 内等待下一个事件。
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Event> {
        EVENTS.recv_timeout(timeout)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe { sys::tls_ble_server_demo_api_deinit() };
    }
}
