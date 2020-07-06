use super::task::*;
use super::w60x_api::*;
use crate::{cstr, wm_println};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use cstr_core::CString;
use cty::{c_int, c_void, size_t};
use spin::Mutex;

#[derive(Clone)]
#[allow(dead_code)]
pub enum Event {
    Connected(i32),
    MessageCome(Vec<u8>),
    SendError,
    Disconnected,
}

#[derive(Clone, Debug, PartialEq)]
enum LwsState {
    ExitSession,
    InitSession,
    SetUpSession,
    HandleSession,
}

#[allow(dead_code)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
}

const CALLBACK_MSG_I32_WRITEABLE: i32 = 1;

extern "C" fn hook<F>(
    wsi: *mut c_void,
    reason: lws_callback_reasons,
    user: *mut c_void,
    _in: *mut c_void,
    len: size_t,
) -> c_int
where
    F: FnMut(*mut c_void, lws_callback_reasons, *mut c_void, *mut c_void, size_t) -> c_int,
{
    // wm_println!("reason={:?}",reason);
    if user.is_null() {
        // wm_println!("user is null!");
        0
    } else {
        let closure = unsafe { &mut *(user as *mut F) };
        closure(wsi, reason, user, _in, len)
    }
}

pub fn get_callback<F>(_closure: &F) -> lws_callback_function
where
    F: FnMut(*mut c_void, lws_callback_reasons, *mut c_void, *mut c_void, size_t) -> c_int,
{
    hook::<F>
}

/// 存放要发送的数据
static mut DATA_LIST: Mutex<BTreeMap<i32, (Vec<u8>, lws_write_protocol)>> =
    Mutex::new(BTreeMap::new());

pub fn connect(
    stk: &Mutex<[u32]>,
    host: &str,
    use_ssl: bool,
    port: i32,
    path: &str,
    notify_task_id: TaskQueueID,
) -> i32 {
    wm_println!("websocket::connect host={} path={}", host, path);
    create("lwsRecvTask", stk, |recv_task| {
        let mut now_state = LwsState::InitSession;
        let mut watch_dog = 0;

        let closure = move |wsi, reason, user_data: *mut c_void, _in, len| {
            let task_ids = unsafe { core::slice::from_raw_parts(user_data as *mut i32, 2) };
            // wm_println!("closure task_ids={:?}", task_ids);
            let recv_task_queue_id = task_ids[0];
            let notify_task_queue_id = task_ids[1];
            match reason {
                lws_callback_reasons::LWS_CALLBACK_CLIENT_ESTABLISHED => {
                    send_msg(LwsState::HandleSession, recv_task_queue_id);
                }

                lws_callback_reasons::LWS_CALLBACK_CLIENT_CONNECTION_ERROR => {
                    send_msg(LwsState::ExitSession, recv_task_queue_id);
                }

                lws_callback_reasons::LWS_CALLBACK_CLOSED => {
                    send_msg(LwsState::ExitSession, recv_task_queue_id);
                }

                lws_callback_reasons::LWS_CALLBACK_CLIENT_RECEIVE => {
                    let buf = unsafe { core::slice::from_raw_parts(_in as *mut _ as *mut u8, len) };
                    send_msg(Event::MessageCome(buf.to_vec()), notify_task_queue_id);
                }
                lws_callback_reasons::LWS_CALLBACK_CLIENT_WRITEABLE => {
                    while let Some((mut data, write_protocol)) =
                        unsafe { DATA_LIST.lock() }.remove(&recv_task_queue_id)
                    {
                        let data_2send = data.as_mut_ptr();
                        let len = data.len();
                        let n = unsafe { lws_write(wsi, data_2send, len, write_protocol) };
                        wm_println!("send data len={} success={}", len, n > 0);
                    }
                    //发送消息必须在发送数据之后调用
                    send_msg(CALLBACK_MSG_I32_WRITEABLE, recv_task_queue_id);
                }
                _ => {}
            }
            0
        };
        let callback = get_callback(&closure);

        let mut conn_info: lws_client_connect_info = unsafe { core::mem::zeroed() };
        let mut info: lws_context_creation_info = unsafe { core::mem::zeroed() };
        let mut lwscontext: *mut c_void = core::ptr::null_mut();
        let mut old_tick = 0;
        //path和host必须生命在loop外边，并且每次连接之前都要重新创建CString
        let host = String::from(host);
        let path = String::from(path);
        let mut _path_cstr = CString::new(path.as_bytes()).unwrap();
        let mut _host_cstr = CString::new(host.as_bytes()).unwrap();
        let mut ws_context: *mut c_void = core::ptr::null_mut();
        //只发送一次连接成功消息
        let mut connected = false;
        let task_ids: &mut [i32; 2] = &mut [recv_task.get_queue_id(), notify_task_id];
        // wm_println!("task_ids={:?}", task_ids);

        let protocols: [lws_protocols; 2] = [
            lws_protocols {
                name: cstr!("ws"),
                callback: Some(callback),
                per_session_data_size: 0,
                rx_buffer_size: 2048,
                id: 0,
                user: core::ptr::null_mut(), // 用户数据
            },
            unsafe { core::mem::zeroed() },
        ];

        loop {
            if let Some(mut msg) = recv_task.rev_msg_wait(HZ / 50) {
                if let Some(state) = msg.downcast_ref::<LwsState>() {
                    //闭包传来的状态信息
                    now_state = state.clone();
                } else if let Some(send_msg) = msg.downcast_mut::<Message>() {
                    //收到外部任务发来的消息，给server发送数据
                    unsafe {
                        DATA_LIST
                            .lock()
                            .insert(recv_task.get_queue_id(), message_to_bytes(send_msg))
                    };
                } else if let Some(i32_msg) = msg.downcast_ref::<i32>() {
                    //闭包传来的其他消息
                    if *i32_msg == CALLBACK_MSG_I32_WRITEABLE {
                        watch_dog = 0;
                        if !connected {
                            //通知任务，连接成功
                            send_msg(Event::Connected(recv_task.get_queue_id()), notify_task_id);
                            connected = true;
                        }
                    }
                } else if let Some(state) = msg.downcast_ref::<Event>() {
                    //转发其他回调函数传递过来的消息
                    send_msg(state.clone(), notify_task_id);
                }
            }

            // wm_println!("{:?}", now_state);
            match now_state {
                LwsState::InitSession => {
                    if lwscontext.is_null() {
                        unsafe { core::ptr::write_bytes(&mut info, 0, 1) };
                        _path_cstr = CString::new(path.as_bytes()).unwrap();
                        _host_cstr = CString::new(host.as_bytes()).unwrap();
                        info.port = -1;
                        if use_ssl {
                            info.options |= 1 << 12;
                        }
                        info.protocols = protocols.as_ptr();
                        info.options = 0;
                        info.max_http_header_data = 512;
                        info.max_http_header_pool = 4;
                        lwscontext = unsafe { lws_create_context(&mut info) };
                        unsafe { core::ptr::write_bytes(&mut conn_info, 0, 1) };
                        conn_info.context = lwscontext;
                        conn_info.address = _host_cstr.as_ptr();
                        conn_info.port = port;
                        //                    connInfo.ssl_connection = LWS_USE_SSL;
                        // let tmp_id = _recv_task_id;
                        conn_info.path = _path_cstr.as_ptr();
                        conn_info.host = _host_cstr.as_ptr();
                        conn_info.userdata = task_ids.as_mut_ptr() as *mut c_void;
                        conn_info.protocol = protocols[0].name;
                        conn_info.ietf_version_or_minus_one = 13;
                        conn_info.origin = _host_cstr.as_ptr();

                        now_state = LwsState::SetUpSession;
                    } else {
                        now_state = LwsState::ExitSession;
                    }
                }

                LwsState::SetUpSession => {
                    ws_context = unsafe { lws_client_connect_via_info(&mut conn_info) };
                    if ws_context.is_null() {
                        wm_println!("websocket connect failed");
                        now_state = LwsState::ExitSession;
                        unsafe { tls_os_time_delay(HZ * 5) };
                    } else {
                        now_state = LwsState::HandleSession;
                    }
                }

                LwsState::HandleSession => {
                    let now_tick = unsafe { tls_os_get_time() };
                    if now_tick - old_tick > HZ {
                        {
                            unsafe { lws_callback_on_writable(ws_context) };
                            if watch_dog > 5 {
                                now_state = LwsState::ExitSession;
                            } else {
                                watch_dog += 1;
                            }
                            old_tick = now_tick;
                        }
                    }
                    unsafe { lws_service(lwscontext, 250) };
                }

                LwsState::ExitSession => {
                    unsafe { tls_os_time_delay(HZ * 5) };
                    if !lwscontext.is_null() {
                        unsafe { lws_context_destroy(lwscontext) };
                    }
                    lwscontext = core::ptr::null_mut();
                    ws_context = core::ptr::null_mut();
                    watch_dog = 0;
                    unsafe { tls_os_time_delay(HZ * 5) };
                    if connected {
                        connected = false;
                        send_msg(Event::Disconnected, notify_task_id);
                    }
                    now_state = LwsState::InitSession;
                }
            }
        }
    })
    .unwrap();
    return 0;
}

pub fn message_to_bytes(msg: &mut Message) -> (Vec<u8>, lws_write_protocol) {
    match msg {
        Message::Text(text) => (text.as_bytes().to_vec(), lws_write_protocol::LWS_WRITE_TEXT),
        Message::Binary(vec) => {
            //移除数据
            (
                vec.drain(..).collect(),
                lws_write_protocol::LWS_WRITE_BINARY,
            )
        }
        Message::Ping => (vec![0x09, 0x00], lws_write_protocol::LWS_WRITE_PING),
        Message::Pong => (vec![0x0A, 0x00], lws_write_protocol::LWS_WRITE_PONG),
    }
}
