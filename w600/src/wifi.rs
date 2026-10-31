//! Wi-Fi STA 模式联网（DHCP 获取地址）。
//!
//! ```no_run
//! let info = w600::wifi::connect("my-ssid", "password", core::time::Duration::from_secs(20))?;
//! w600::println!("ip = {}", info.ip);
//! # Ok::<(), w600::wifi::Error>(())
//! ```

use core::net::Ipv4Addr;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use crate::sync::Channel;
use crate::sys;
use crate::time::Instant;

/// 网络状态变化事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// 已加入 AP（尚未获得 IP）。
    Associated,
    /// 加入 AP 失败（密码错误、找不到 AP 等）。
    AssociationFailed,
    /// 与 AP 断开。
    Disconnected,
    /// DHCP 已分配 IP，可以开始通信。
    GotIp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// SSID 为空或超过 32 字节、密码超过 64 字节。
    InvalidArgument,
    /// 加入 AP 失败。
    ConnectFailed,
    /// 在超时时间内没有拿到 IP。
    Timeout,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Error::InvalidArgument => "invalid ssid or password",
            Error::ConnectFailed => "failed to join access point",
            Error::Timeout => "timed out waiting for ip address",
        })
    }
}

impl core::error::Error for Error {}

/// 当前的 IPv4 配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpInfo {
    pub ip: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub dns: Ipv4Addr,
}

static EVENTS: Channel<Event> = Channel::new(8);
static CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn status_changed(status: u8) {
    let event = match status as u32 {
        sys::NETIF_WIFI_JOIN_SUCCESS => Event::Associated,
        sys::NETIF_WIFI_JOIN_FAILED => Event::AssociationFailed,
        sys::NETIF_WIFI_DISCONNECTED => Event::Disconnected,
        sys::NETIF_IP_NET_UP => Event::GotIp,
        _ => return,
    };
    // 没人读取时队列会满，丢掉最新的事件即可。
    let _ = EVENTS.send(event);
}

fn ensure_callback() {
    if !CALLBACK_REGISTERED.swap(true, Ordering::AcqRel) {
        EVENTS.init();
        unsafe { sys::tls_netif_add_status_event(Some(status_changed)) };
    }
}

/// lwIP 按网络字节序存放地址，小端 CPU 上 `to_le_bytes` 恰好得到 a.b.c.d 的顺序。
fn ipv4(addr: &sys::ip_addr_t) -> Ipv4Addr {
    Ipv4Addr::from(addr.addr.to_le_bytes())
}

/// 开始连接，不等待结果；之后可以用 [`wait_event`] 观察进展。
pub fn begin_connect(ssid: &str, password: &str) -> Result<(), Error> {
    if ssid.is_empty() || ssid.len() > 32 || password.len() > 64 {
        return Err(Error::InvalidArgument);
    }
    ensure_callback();
    while EVENTS.recv_timeout(Duration::ZERO).is_some() {}

    unsafe {
        sys::tls_wifi_disconnect();

        // 最后一个参数（是否读写 Flash）在 W600 SDK 中是 u8，在 W800 SDK 中是 bool。
        let mut protocol: u8 = 0;
        sys::tls_param_get(sys::TLS_PARAM_ID_WPROTOCOL as i32, (&raw mut protocol).cast(), true as _);
        if protocol as u32 != sys::TLS_PARAM_IEEE80211_INFRA {
            sys::tls_wifi_softap_destroy();
            protocol = sys::TLS_PARAM_IEEE80211_INFRA as u8;
            sys::tls_param_set(sys::TLS_PARAM_ID_WPROTOCOL as i32, (&raw mut protocol).cast(), false as _);
        }
        sys::tls_wifi_set_oneshot_flag(0);

        let mut ip = sys::tls_param_ip::default();
        sys::tls_param_get(sys::TLS_PARAM_ID_IP as i32, (&raw mut ip).cast(), false as _);
        ip.dhcp_enable = 1;
        sys::tls_param_set(sys::TLS_PARAM_ID_IP as i32, (&raw mut ip).cast(), false as _);

        // SDK 按长度读取，不要求结尾的 NUL，也不会修改缓冲区。
        let ret = sys::tls_wifi_connect(
            ssid.as_ptr().cast_mut(),
            ssid.len() as u8,
            password.as_ptr().cast_mut(),
            password.len() as u8,
        );
        if ret != 0 {
            return Err(Error::InvalidArgument);
        }
    }
    Ok(())
}

/// 连接 AP 并等待 DHCP 分配地址。开放网络的密码传空字符串。
pub fn connect(ssid: &str, password: &str, timeout: Duration) -> Result<IpInfo, Error> {
    begin_connect(ssid, password)?;
    let start = Instant::now();
    loop {
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Err(Error::Timeout);
        }
        match EVENTS.recv_timeout(remaining) {
            Some(Event::GotIp) => return ip_info().ok_or(Error::ConnectFailed),
            Some(Event::AssociationFailed) => return Err(Error::ConnectFailed),
            Some(_) => {}
            None => return Err(Error::Timeout),
        }
    }
}

/// 等待下一个网络事件；`None` 表示一直等待。可用于检测掉线后重连。
pub fn wait_event(timeout: Option<Duration>) -> Option<Event> {
    ensure_callback();
    match timeout {
        None => Some(EVENTS.recv()),
        Some(t) => EVENTS.recv_timeout(t),
    }
}

pub fn disconnect() {
    unsafe { sys::tls_wifi_disconnect() };
}

/// 已加入 AP 并且拿到了 IP。
pub fn is_connected() -> bool {
    let joined = unsafe { sys::tls_wifi_get_state() } == sys::tls_wifi_states::WM_WIFI_JOINED;
    joined && ip_info().is_some()
}

/// 当前的 IP 配置；尚未获得地址时返回 `None`。
pub fn ip_info() -> Option<IpInfo> {
    let ethif = unsafe { sys::tls_netif_get_ethif() };
    if ethif.is_null() {
        return None;
    }
    let ethif = unsafe { &*ethif };
    if ethif.ip_addr.addr == 0 {
        return None;
    }
    Some(IpInfo {
        ip: ipv4(&ethif.ip_addr),
        netmask: ipv4(&ethif.netmask),
        gateway: ipv4(&ethif.gw),
        dns: ipv4(&ethif.dns1),
    })
}
