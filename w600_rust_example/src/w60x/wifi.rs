use super::task::*;
use super::w60x_api::*;
use crate::wm_println;
// use alloc::boxed::Box;
use alloc::string::String;
use core::ffi::c_void;
use cstr_core::CString;
use spin::Mutex;

// TaskQueueID无效不会收到消息
static NOTIFY_TASK: Mutex<Option<TaskQueueID>> = Mutex::new(None);

#[derive(Debug)]
pub enum State {
    JoinSuccess,
    JoinFailed,
    Disconnected,
    IpNetUp(String),
    Unknown(u8),
}

fn get_status(status: u8) -> State {
    match status {
        NETIF_WIFI_JOIN_SUCCESS => State::JoinSuccess,
        NETIF_WIFI_JOIN_FAILED => State::JoinFailed,
        NETIF_WIFI_DISCONNECTED => State::Disconnected,
        NETIF_IP_NET_UP => {
            let tmpethif: &mut tls_ethif = unsafe { &mut *tls_netif_get_ethif() };
            let bytes = tmpethif.ip_addr.addr.to_le_bytes();
            State::IpNetUp(format!(
                "{}.{}.{}.{}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            ))
        }
        _ => State::Unknown(status),
    }
}

extern "C" fn wifi_status_callback(status: u8) {
    let status = get_status(status);
    // wm_println!("wifi_status_callback status={:?}", status);
    if let Some(task_queue_ptr) = NOTIFY_TASK.lock().as_ref() {
        send_msg(status, *task_queue_ptr);
        return;
    }
    wm_println!("notify task is None {:?}", status);
}

#[allow(dead_code)]
pub fn is_network_ok() -> bool {
    let tmpethif = unsafe { tls_netif_get_ethif() };
    let wifi_state = unsafe { tls_wifi_get_state() };
    if let tls_wifi_states::WM_WIFI_JOINED = wifi_state {
        if !tmpethif.is_null() {
            if (unsafe { &*tmpethif }).ip_addr.addr != 0 {
                return true;
            }
        }
    }
    false
}

/// 连接WIFI 注意: 一次只能发送通知到一个OSTask, WIFI状态变化后会给OSTask发送一个wifi::Status消息
pub fn connect(notify_task: Option<TaskQueueID>, ssid: &str, pwd: &str) -> Result<(), String> {
    if let Some(task) = notify_task {
        *NOTIFY_TASK.lock() = Some(task);
    }

    let mut ip_param = tls_param_ip::default();
    let mut wireless_protocol: u8 = 0;

    // wm_println!("\nssid:{}", ssid);
    // wm_println!("password={}", pwd);
    unsafe { tls_wifi_disconnect() };

    if unsafe {
        tls_param_get(
            TLS_PARAM_ID_WPROTOCOL,
            &mut wireless_protocol as *mut u8 as *mut c_void,
            true,
        )
    } != TLS_PARAM_STATUS_OK
    {
        wm_println!("tls_param_get error: TLS_PARAM_ID_WPROTOCOL");
    }

    if TLS_PARAM_IEEE80211_INFRA != wireless_protocol as i32 {
        unsafe {
            tls_wifi_softap_destroy();
        }
        wireless_protocol = TLS_PARAM_IEEE80211_INFRA as u8;
        if unsafe {
            tls_param_set(
                TLS_PARAM_ID_WPROTOCOL,
                &mut wireless_protocol as *mut u8 as *mut c_void,
                false,
            )
        } != TLS_PARAM_STATUS_OK
        {
            wm_println!("tls_param_set error: TLS_PARAM_ID_WPROTOCOL");
        }
    }

    unsafe {
        tls_wifi_set_oneshot_flag(0);
    }

    if unsafe {
        tls_param_get(
            TLS_PARAM_ID_IP,
            &mut ip_param as *mut _ as *mut c_void,
            false,
        )
    } != TLS_PARAM_STATUS_OK
    {
        wm_println!("tls_param_get error: tls_param_ip");
    }
    ip_param.dhcp_enable = TRUE as u8;
    if unsafe {
        tls_param_set(
            TLS_PARAM_ID_IP,
            &mut ip_param as *mut _ as *mut c_void,
            false,
        )
    } != TLS_PARAM_STATUS_OK
    {
        wm_println!("tls_param_set error: tls_param_ip");
    }

    if unsafe { tls_netif_add_status_event(wifi_status_callback) } != 0 {
        wm_println!("tls_netif_add_status_event error");
    }

    if unsafe {
        tls_wifi_connect(
            CString::new(ssid.as_bytes()).unwrap().as_ptr(),
            ssid.len() as u8,
            CString::new(pwd.as_bytes()).unwrap().as_ptr(),
            pwd.len() as u8,
        )
    } != WM_SUCCESS
    {
        Err(String::from("parameter wrong"))
    } else {
        Ok(())
    }
}
