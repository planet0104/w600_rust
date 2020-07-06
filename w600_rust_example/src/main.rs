#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(const_btree_new)]

#[macro_use]
extern crate alloc;

mod w60x;
use alloc::string::String;
use core::{alloc::Layout, panic::PanicInfo};
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use w60x::{gpio::*, pwm::*, task, websocket, wifi};

const HEAP_SIZE: usize = 1024 * 16;
static mut HEAP: &mut [u8; HEAP_SIZE] = &mut [0u8; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static TASK_STK: Mutex<[u32; 4096]> = Mutex::new([0; 4096]);
static WS_TASK_STK: Mutex<[u32; 2048]> = Mutex::new([0; 2048]);

fn init_led() {
    let freq = 1000;
    let duty = 255;
    let pnum = 0;
    // R
    pwm3_config(WM_IO_PB_16);
    pwm_init(CHANNEL_PWM3, freq, duty, pnum);
    pwm_start(CHANNEL_PWM3);
    // B
    pwm2_config(WM_IO_PB_17);
    pwm_init(CHANNEL_PWM2, freq, duty, pnum);
    pwm_start(CHANNEL_PWM2);
    // G
    pwm1_config(WM_IO_PB_18);
    pwm_init(CHANNEL_PWM1, freq, duty, pnum);
    pwm_start(CHANNEL_PWM1);
}

fn set_led_color(r: u8, g: u8, b: u8) {
    pwm_duty_set(CHANNEL_PWM3, 255 - r);
    pwm_duty_set(CHANNEL_PWM2, 255 - b);
    pwm_duty_set(CHANNEL_PWM1, 255 - g);
}

#[no_mangle]
pub extern "C" fn UserMain() {
    // 初始化内存分配器
    unsafe { ALLOCATOR.lock().init(HEAP.as_mut_ptr() as usize, HEAP_SIZE) }

    wm_println!("user main start.");

    init_led();

    //创建任务
    task::create("main", &TASK_STK, |task| {
        wm_println!("[{}] task create success", task.get_name());

        let ssid = "CMCC-Vuf6";
        let pass = "fcpsn357";
        let ws_host = "192.168.1.7";
        let ws_port = 9001;
        let client_id = "clientx11";

        //连接wifi
        match wifi::connect(Some(task.get_queue_id()), ssid, pass) {
            Ok(()) => {
                wm_println!("connecting wifi...");
            }
            Err(err) => {
                wm_println!("wifi connect failed: {}", err);
            }
        }

        //消息接收消息
        loop {
            // 消息是一个Any类型
            if let Some(msg) = task.rev_msg() {
                if let Some(state) = msg.downcast_ref::<wifi::State>() {
                    match state {
                        wifi::State::IpNetUp(ip) => {
                            wm_println!("ip: {}", ip);
                            wm_println!("connect websocket server...");
                            let _ret = websocket::connect(
                                &WS_TASK_STK,
                                ws_host,
                                false,
                                ws_port,
                                "/",
                                task.get_queue_id(),
                            );
                        }
                        other => {
                            wm_println!("wifi: {:?}", other);
                        }
                    }
                } else if let Some(state) = msg.downcast_ref::<websocket::Event>() {
                    match state {
                        websocket::Event::Disconnected => {
                            wm_println!("websocket disconnected!");
                        }
                        websocket::Event::Connected(ws_task_queue) => {
                            wm_println!("websocket connected");
                            //注册设备
                            task::send_msg(
                                websocket::Message::Text(String::from(client_id)),
                                *ws_task_queue,
                            );
                        }
                        websocket::Event::MessageCome(msg) => {
                            if msg.len() == 3 {
                                wm_println!("msg come: {:?}", msg);
                                set_led_color(msg[0], msg[1], msg[2]);
                            } else {
                                wm_println!("msg come: {:?}", String::from_utf8_lossy(msg));
                            }
                        }
                        websocket::Event::SendError => {
                            wm_println!("websocket msg send error!");
                        }
                    }
                }
            }
        }

        //任务结束
    })
    .unwrap();
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    wm_println!("panic occurred: {:?}", panic_info);
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    wm_println!("Out of Memory!!!");
    loop {}
}
