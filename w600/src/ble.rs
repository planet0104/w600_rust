//! BLE 从机（仅 W800）：广播设备名，接受手机等主机连接，并通过同一个特征收发数据，
//! 与 HM-10 等串口透传模块的服务相同，蓝牙调试工具可以在一个特征上边发边收。
//!
//! | UUID | 属性 | 用途 |
//! | --- | --- | --- |
//! | `0xFFE0` | 服务 | 广播中也包含此 UUID |
//! | `0xFFE1` | Write / Write Without Response | 主机写入的数据以 [`Event::Received`] 交给应用 |
//! | `0xFFE1` | Notify | [`Server::notify`] 向主机发送数据（主机订阅通知后才会显示） |
//! | `0xFFE1` | Read | 读到最后一次 [`Server::notify`] 发送的数据 |
//!
//! ```no_run
//! let server = w600::ble::Server::start("W800-Rust").unwrap();
//! loop {
//!     if let w600::ble::Event::Received(data) = server.recv() {
//!         let _ = server.notify(&data);
//!     }
//! }
//! ```

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU16, AtomicU32, Ordering};
use core::time::Duration;

use crate::sync::{Channel, Mutex};
use crate::sys::{self, ble_gap_event, ble_gatt_access_ctxt, ble_gatt_chr_def, ble_gatt_svc_def, ble_uuid16_t};

const SERVICE_UUID: u16 = 0xFFE0;
const CHARACTERISTIC_UUID: u16 = 0xFFE1;

/// HCI 断开原因：本端用户终止连接。
const BLE_ERR_REM_USER_CONN_TERM: u8 = 0x13;
const NO_CONNECTION: u32 = u32::MAX;

/// 广播包最多 31 字节，扣除标志（3）、服务 UUID（4）和名字字段头（2）后剩余 22 字节。
pub const MAX_NAME_LEN: usize = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 设备名为空、超过 [`MAX_NAME_LEN`] 字节或包含 NUL。
    InvalidName,
    /// 服务已经启动。
    AlreadyStarted,
    /// 当前没有主机连接。
    NotConnected,
    /// 数据为空或超过 [`Server::max_notify_len`]。
    InvalidLength,
    /// NimBLE / SDK 返回的错误码。
    Stack(i32),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InvalidName => f.write_str("invalid ble device name"),
            Error::AlreadyStarted => f.write_str("ble server already started"),
            Error::NotConnected => f.write_str("no ble central connected"),
            Error::InvalidLength => f.write_str("notify data is empty or longer than the mtu"),
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
    /// 主机断开（HCI 断开原因码），之后会自动重新广播。
    Disconnected { reason: u8 },
    /// 主机向 `0xFFE1` 写入的数据。
    Received(Vec<u8>),
}

static EVENTS: Channel<Event> = Channel::new(16);
static STARTED: AtomicBool = AtomicBool::new(false);
static CONNECTION: AtomicU32 = AtomicU32::new(NO_CONNECTION);
/// 特征值的属性句柄，由 NimBLE 在 `ble_gatts_start` 时写入。
static VALUE_HANDLE: AtomicU16 = AtomicU16::new(0);
static LAST_SENT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// NimBLE 保存的是指针，GATT 表与监听器必须一直有效：首次启动时创建，之后复用。
struct Gatt {
    service_uuid: ble_uuid16_t,
    characteristic_uuid: ble_uuid16_t,
    characteristics: [ble_gatt_chr_def; 2],
    services: [ble_gatt_svc_def; 2],
    listener: sys::ble_gap_event_listener,
}

static GATT: AtomicPtr<Gatt> = AtomicPtr::new(ptr::null_mut());

fn uuid16(value: u16) -> ble_uuid16_t {
    ble_uuid16_t { u: sys::ble_uuid_t { type_: sys::BLE_UUID_TYPE_16.0 as u8 }, value }
}

fn gatt() -> *mut Gatt {
    let existing = GATT.load(Ordering::Acquire);
    if !existing.is_null() {
        return existing;
    }
    // 结构体末尾的全零项是 NimBLE 要求的结束标记。
    let gatt = Box::into_raw(Box::new(Gatt {
        service_uuid: uuid16(SERVICE_UUID),
        characteristic_uuid: uuid16(CHARACTERISTIC_UUID),
        characteristics: unsafe { core::mem::zeroed() },
        services: unsafe { core::mem::zeroed() },
        listener: unsafe { core::mem::zeroed() },
    }));
    unsafe {
        let g = &mut *gatt;
        g.characteristics[0] = ble_gatt_chr_def {
            uuid: &raw const g.characteristic_uuid.u,
            access_cb: Some(on_access),
            flags: (sys::BLE_GATT_CHR_F_READ
                | sys::BLE_GATT_CHR_F_WRITE
                | sys::BLE_GATT_CHR_F_WRITE_NO_RSP
                | sys::BLE_GATT_CHR_F_NOTIFY) as u16,
            val_handle: VALUE_HANDLE.as_ptr(),
            ..core::mem::zeroed()
        };
        g.services[0] = ble_gatt_svc_def {
            type_: sys::BLE_GATT_SVC_TYPE_PRIMARY as u8,
            uuid: &raw const g.service_uuid.u,
            characteristics: g.characteristics.as_ptr(),
            ..core::mem::zeroed()
        };
    }
    GATT.store(gatt, Ordering::Release);
    gatt
}

/// 在 NimBLE 的主机任务中调用。没人读取时队列会满，丢掉新的事件。
unsafe extern "C" fn on_access(_conn: u16, _attr: u16, ctxt: *mut ble_gatt_access_ctxt, _arg: *mut c_void) -> c_int {
    let ctxt = unsafe { &*ctxt };
    match ctxt.op as u32 {
        sys::BLE_GATT_ACCESS_OP_WRITE_CHR => {
            let mut data = Vec::new();
            let mut om = ctxt.om;
            while let Some(m) = unsafe { om.as_ref() } {
                data.extend_from_slice(unsafe { core::slice::from_raw_parts(m.om_data, m.om_len as usize) });
                om = m.om_next.sle_next;
            }
            let _ = EVENTS.send(Event::Received(data));
            0
        }
        sys::BLE_GATT_ACCESS_OP_READ_CHR => {
            let value = LAST_SENT.lock();
            match unsafe { sys::os_mbuf_append(ctxt.om, value.as_ptr().cast(), value.len() as u16) } {
                0 => 0,
                _ => sys::BLE_ATT_ERR_INSUFFICIENT_RES as c_int,
            }
        }
        _ => sys::BLE_ATT_ERR_UNLIKELY as c_int,
    }
}

/// 在 NimBLE 的主机任务中调用。
unsafe extern "C" fn on_gap_event(event: *mut ble_gap_event, _arg: *mut c_void) -> c_int {
    let event = unsafe { &*event };
    match event.type_ as u32 {
        sys::BLE_GAP_EVENT_CONNECT => {
            let connect = unsafe { event.__bindgen_anon_1.connect };
            if connect.status == 0 {
                CONNECTION.store(connect.conn_handle as u32, Ordering::Release);
                let _ = EVENTS.send(Event::Connected);
            } else if STARTED.load(Ordering::Acquire) {
                advertise();
            }
        }
        sys::BLE_GAP_EVENT_DISCONNECT => {
            CONNECTION.store(NO_CONNECTION, Ordering::Release);
            // NimBLE 的原因码为 0x200 + HCI 原因码。
            let reason = unsafe { event.__bindgen_anon_1.disconnect.reason } as u8;
            let _ = EVENTS.send(Event::Disconnected { reason });
            if STARTED.load(Ordering::Acquire) {
                advertise();
            }
        }
        _ => {}
    }
    0
}

/// 开始可连接广播（广播数据已在启动时设置）。
fn advertise() -> c_int {
    let rc = unsafe { sys::tls_nimble_gap_adv(sys::wm_ble_adv_type_t::WM_BLE_ADV_IND, 0) };
    if rc == 0 {
        let _ = EVENTS.send(Event::Advertising);
    }
    rc
}

fn check(rc: c_int) -> Result<(), Error> {
    if rc == 0 { Ok(()) } else { Err(Error::Stack(rc)) }
}

/// GATT 服务端。同一时间只能有一个，丢弃时停止广播并断开连接（协议栈保持运行）。
pub struct Server {
    _private: (),
}

impl Server {
    /// 启动蓝牙协议栈（首次调用时，约需数百毫秒），注册服务并以 `name` 为设备名开始广播。
    pub fn start(name: &str) -> Result<Server, Error> {
        if name.is_empty() || name.len() > MAX_NAME_LEN || name.contains('\0') {
            return Err(Error::InvalidName);
        }
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(Error::AlreadyStarted);
        }
        let result = unsafe { Self::init(name) };
        if result.is_err() {
            STARTED.store(false, Ordering::Release);
        }
        result.map(|()| Server { _private: () })
    }

    unsafe fn init(name: &str) -> Result<(), Error> {
        EVENTS.init();
        let rc = unsafe { sys::tls_bt_init(0xFF) };
        if rc != sys::BLE_HS_EALREADY as c_int {
            check(rc)?;
        }

        let mut c_name = [0u8; MAX_NAME_LEN + 1];
        c_name[..name.len()].copy_from_slice(name.as_bytes());
        let gatt = gatt();
        unsafe {
            check(sys::ble_svc_gap_device_name_set(c_name.as_ptr().cast()))?;
            check(sys::ble_gatts_reset())?;
            check(sys::ble_gatts_count_cfg((*gatt).services.as_ptr()))?;
            check(sys::ble_gatts_add_svcs((*gatt).services.as_ptr()))?;
            check(sys::ble_gatts_start())?;
            // 上次启动在注册之后失败时，监听器仍在列表中。
            let rc = sys::ble_gap_event_listener_register(&raw mut (*gatt).listener, Some(on_gap_event), ptr::null_mut());
            if rc != sys::BLE_HS_EALREADY as c_int {
                check(rc)?;
            }
        }

        // 广播数据：标志（LE 通用可发现、不支持 BR/EDR）、16 位服务 UUID、完整设备名。
        let mut adv = Vec::with_capacity(31);
        adv.extend_from_slice(&[0x02, 0x01, 0x06]);
        adv.extend_from_slice(&[0x03, 0x03, SERVICE_UUID as u8, (SERVICE_UUID >> 8) as u8]);
        adv.extend_from_slice(&[name.len() as u8 + 1, 0x09]);
        adv.extend_from_slice(name.as_bytes());
        unsafe {
            check(sys::tls_ble_gap_set_data(sys::wm_ble_gap_data_t::WM_BLE_ADV_DATA, adv.as_mut_ptr(), adv.len() as u8))?;
        }
        check(advertise())
    }

    /// 等待下一个事件。
    pub fn recv(&self) -> Event {
        EVENTS.recv()
    }

    /// 在 `timeout` 内等待下一个事件。
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Event> {
        EVENTS.recv_timeout(timeout)
    }

    /// 当前是否有主机连接。
    pub fn is_connected(&self) -> bool {
        CONNECTION.load(Ordering::Acquire) != NO_CONNECTION
    }

    /// 单次 [`notify`](Self::notify) 最多发送的字节数（ATT MTU - 3）：连接后为 20，
    /// 主机协商了更大的 MTU 后随之增大。
    pub fn max_notify_len(&self) -> usize {
        let conn = CONNECTION.load(Ordering::Acquire);
        let mtu = if conn == NO_CONNECTION { 0 } else { unsafe { sys::ble_att_mtu(conn as u16) } };
        (mtu.max(23) - 3) as usize
    }

    /// 通过 `0xFFE1` 向主机发送一条通知。主机没有订阅通知时，数据会被主机忽略。
    pub fn notify(&self, data: &[u8]) -> Result<(), Error> {
        let conn = CONNECTION.load(Ordering::Acquire);
        if conn == NO_CONNECTION {
            return Err(Error::NotConnected);
        }
        if data.is_empty() || data.len() > self.max_notify_len() {
            return Err(Error::InvalidLength);
        }
        let om = unsafe { sys::ble_hs_mbuf_from_flat(data.as_ptr().cast(), data.len() as u16) };
        if om.is_null() {
            return Err(Error::Stack(sys::BLE_HS_ENOMEM as i32));
        }
        // 无论成功与否，mbuf 都由协议栈释放。
        check(unsafe { sys::ble_gattc_notify_custom(conn as u16, VALUE_HANDLE.load(Ordering::Acquire), om) })?;
        *LAST_SENT.lock() = data.to_vec();
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        STARTED.store(false, Ordering::Release);
        unsafe {
            sys::tls_nimble_gap_adv(sys::wm_ble_adv_type_t::WM_BLE_ADV_STOP, 0);
            let conn = CONNECTION.load(Ordering::Acquire);
            if conn != NO_CONNECTION {
                sys::ble_gap_terminate(conn as u16, BLE_ERR_REM_USER_CONN_TERM);
            }
            sys::ble_gap_event_listener_unregister(&raw mut (*GATT.load(Ordering::Acquire)).listener);
        }
    }
}
