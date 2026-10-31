//! 线程，用法与 `std::thread` 相近。
//!
//! W600 SDK 里的 FreeRTOS 删除任务时会把任务栈交给 heap_2 的 `vPortFree` 释放，
//! 而任务栈并不是从 heap_2 分配的，删除就会破坏堆。因此这里的线程执行完闭包后不会被删除，
//! 而是作为空闲工作线程等待下一次 [`spawn`]，复用它的栈和优先级（W800 上也采用同样的方式）。
//!
//! 优先级沿用 SDK 的约定：**数值越小优先级越高**。SDK 内部任务占用 0～28，
//! 应用线程默认使用 [`DEFAULT_PRIORITY`]，相同优先级的线程按时间片轮转。

use core::ffi::c_void;
use core::time::Duration;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use crate::sync::{Channel, Mutex};
use crate::sys;
use crate::time::ticks;

/// 默认栈大小（字节）。
pub const DEFAULT_STACK_SIZE: usize = 4 * 1024;
/// 默认优先级。
pub const DEFAULT_PRIORITY: u8 = 40;
/// 允许的最高（数值最小）优先级，更高的优先级保留给 SDK 内部任务。
pub const MIN_PRIORITY_VALUE: u8 = 30;
/// 允许的最低（数值最大）优先级，63 是 FreeRTOS 的空闲任务。
pub const MAX_PRIORITY_VALUE: u8 = 62;
const MIN_STACK_SIZE: usize = 1024;

type Job = Box<dyn FnOnce() + Send>;

struct Worker {
    inbox: Channel<Job>,
    stack_size: usize,
    priority: u8,
}

static IDLE_WORKERS: Mutex<Vec<&'static Worker>> = Mutex::new(Vec::new());

/// 创建线程失败：通常是内存不足，或（W600 上）任务栈超出了 SDK 要求的地址范围（0x20028000 以下）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnError;

impl core::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("failed to create thread")
    }
}

impl core::error::Error for SpawnError {}

/// 线程配置。
#[derive(Debug, Clone)]
pub struct Builder {
    name: &'static str,
    stack_size: usize,
    priority: u8,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self { name: "rust", stack_size: DEFAULT_STACK_SIZE, priority: DEFAULT_PRIORITY }
    }

    /// 任务名，只在新建底层任务时使用（复用空闲线程时保留原名），最长 15 字节。
    pub fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// 栈大小（字节），最小 1KB。
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = size.max(MIN_STACK_SIZE);
        self
    }

    /// 优先级，数值越小越优先，范围 [`MIN_PRIORITY_VALUE`]..=[`MAX_PRIORITY_VALUE`]。
    pub fn priority(mut self, priority: u8) -> Self {
        assert!((MIN_PRIORITY_VALUE..=MAX_PRIORITY_VALUE).contains(&priority), "thread priority out of range");
        self.priority = priority;
        self
    }

    pub fn spawn<F, T>(self, f: F) -> Result<JoinHandle<T>, SpawnError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let result = Arc::new(Channel::new(1));
        let tx = result.clone();
        let job: Job = Box::new(move || {
            let _ = tx.send(f());
        });

        let worker = {
            let mut idle = IDLE_WORKERS.lock();
            idle.iter()
                .position(|w| w.stack_size >= self.stack_size && w.priority == self.priority)
                .map(|i| idle.swap_remove(i))
        };
        let worker = match worker {
            Some(w) => w,
            None => Worker::start(self.name, self.stack_size, self.priority)?,
        };
        if worker.inbox.send(job).is_err() {
            unreachable!("idle worker inbox is full");
        }
        Ok(JoinHandle { result })
    }
}

impl Worker {
    fn start(name: &str, stack_size: usize, priority: u8) -> Result<&'static Worker, SpawnError> {
        let worker = Box::new(Worker { inbox: Channel::new(1), stack_size, priority });
        worker.inbox.init();
        let stack = vec![0u32; stack_size / 4].into_boxed_slice();

        let mut c_name = [0u8; 16];
        let len = name.len().min(c_name.len() - 1);
        c_name[..len].copy_from_slice(&name.as_bytes()[..len]);

        let worker = Box::into_raw(worker);
        let stack = Box::into_raw(stack);
        let status = unsafe {
            sys::tls_os_task_create(
                core::ptr::null_mut(),
                c_name.as_ptr().cast(),
                Some(worker_main),
                worker.cast(),
                stack.cast(),
                (stack_size / 4 * 4) as u32,
                priority as u32,
                0,
            )
        };
        if status != sys::tls_os_status::TLS_OS_SUCCESS {
            unsafe {
                drop(Box::from_raw(stack));
                drop(Box::from_raw(worker));
            }
            return Err(SpawnError);
        }
        Ok(unsafe { &*worker })
    }
}

unsafe extern "C" fn worker_main(param: *mut c_void) {
    let worker: &'static Worker = unsafe { &*param.cast::<Worker>() };
    loop {
        let job = worker.inbox.recv();
        job();
        IDLE_WORKERS.lock().push(worker);
    }
}

/// 用默认配置创建线程。
///
/// # Panics
///
/// 创建失败时 panic，需要处理错误时使用 [`Builder::spawn`]。
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(f).expect("failed to spawn thread")
}

/// 线程句柄。丢弃句柄不会停止线程。
pub struct JoinHandle<T> {
    result: Arc<Channel<T>>,
}

impl<T> JoinHandle<T> {
    /// 等待线程结束并取得返回值。
    pub fn join(self) -> T {
        self.result.recv()
    }

    /// 在 `timeout` 内等待线程结束；超时则把句柄退回。
    pub fn join_timeout(self, timeout: Duration) -> Result<T, Self> {
        self.result.recv_timeout(timeout).ok_or(self)
    }
}

/// 让当前线程休眠（精度 2ms，向上取整）。
pub fn sleep(duration: Duration) {
    unsafe { sys::tls_os_time_delay(ticks(duration)) };
}

/// 让出 CPU 给同优先级的其他线程。
pub fn yield_now() {
    unsafe { sys::tls_os_time_delay(0) };
}
