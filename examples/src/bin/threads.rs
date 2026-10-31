//! 线程、互斥锁与消息通道。

#![no_std]
#![no_main]

extern crate alloc;

use core::time::Duration;

use alloc::vec::Vec;
use w600::sync::{Arc, Channel, Mutex};
use w600::{println, thread};

w600::entry!(main);

static COUNTER: Mutex<u32> = Mutex::new(0);

fn main() {
    let results = Arc::new(Channel::new(4));

    let workers: Vec<_> = (0..4u32)
        .map(|id| {
            let results = results.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    *COUNTER.lock() += 1;
                }
                thread::sleep(Duration::from_millis(10 * id as u64));
                let _ = results.send(id);
                id * 10
            })
        })
        .collect();

    for _ in 0..4 {
        println!("worker {} finished", results.recv());
    }
    let sum: u32 = workers.into_iter().map(|h| h.join()).sum();
    println!("counter = {}, sum of results = {}", *COUNTER.lock(), sum);

    // 已结束的线程会被复用，不会再分配新的栈。
    let h = thread::spawn(|| "reused worker");
    println!("{}", h.join());
}
