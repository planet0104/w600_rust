//! 基于 FreeRTOS 的同步原语。
//!
//! [`Mutex`]、[`Semaphore`]、[`Channel`] 的构造函数都是 `const fn`，可以直接放在 `static` 里；
//! 底层 OS 对象在第一次使用时创建，因此**第一次使用必须在线程中**，不能在中断里。

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use core::time::Duration;

use alloc::boxed::Box;
pub use alloc::sync::{Arc, Weak};

use crate::sys;
use crate::time::ticks;

fn ok(status: sys::tls_os_status_t) -> bool {
    status == sys::tls_os_status::TLS_OS_SUCCESS
}

/// 延迟创建的 OS 对象句柄；多个线程同时初始化时只保留一个。
struct LazyHandle(AtomicPtr<c_void>);

impl LazyHandle {
    const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    fn get(&self, create: impl FnOnce() -> *mut c_void, destroy: impl FnOnce(*mut c_void)) -> *mut c_void {
        let p = self.0.load(Ordering::Acquire);
        if !p.is_null() {
            return p;
        }
        assert!(!crate::in_interrupt(), "sync object must be first used from a thread");
        let new = create();
        assert!(!new.is_null(), "failed to create OS object");
        match self.0.compare_exchange(ptr::null_mut(), new, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => new,
            Err(existing) => {
                destroy(new);
                existing
            }
        }
    }

    fn existing(&mut self) -> Option<*mut c_void> {
        let p = *self.0.get_mut();
        (!p.is_null()).then_some(p)
    }
}

/// 互斥锁（FreeRTOS mutex，带优先级继承，不可重入），只能在线程中使用。
pub struct Mutex<T: ?Sized> {
    raw: LazyHandle,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self { raw: LazyHandle::new(), data: UnsafeCell::new(value) }
    }
}

impl<T: ?Sized> Mutex<T> {
    fn handle(&self) -> *mut c_void {
        self.raw.get(
            || {
                let mut h = ptr::null_mut();
                unsafe { sys::tls_os_mutex_create(0, &mut h) };
                h
            },
            |h| unsafe {
                sys::tls_os_mutex_delete(h);
            },
        )
    }

    /// 加锁，必要时一直等待。同一线程重复加锁会死锁。
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let ok = ok(unsafe { sys::tls_os_mutex_acquire(self.handle(), 0) });
        assert!(ok, "mutex acquire failed");
        MutexGuard { mutex: self, _not_send: PhantomData }
    }

    /// 在 `timeout` 内尝试加锁（最小等待 2ms）。
    pub fn lock_timeout(&self, timeout: Duration) -> Option<MutexGuard<'_, T>> {
        ok(unsafe { sys::tls_os_mutex_acquire(self.handle(), ticks(timeout)) })
            .then_some(MutexGuard { mutex: self, _not_send: PhantomData })
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        if let Some(h) = self.raw.existing() {
            unsafe { sys::tls_os_mutex_delete(h) };
        }
    }
}

/// FreeRTOS 要求由加锁的线程解锁，所以守卫不能跨线程传递。
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
    _not_send: PhantomData<*const ()>,
}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { sys::tls_os_mutex_release(self.mutex.handle()) };
    }
}

/// 计数信号量。`release` 可以在中断里调用。SDK 把计数上限固定为 5。
pub struct Semaphore {
    raw: LazyHandle,
    initial: u32,
}

unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl Semaphore {
    pub const fn new(initial: u32) -> Self {
        Self { raw: LazyHandle::new(), initial }
    }

    fn handle(&self) -> *mut c_void {
        self.raw.get(
            || {
                let mut h = ptr::null_mut();
                unsafe { sys::tls_os_sem_create(&mut h, self.initial) };
                h
            },
            |h| unsafe {
                sys::tls_os_sem_delete(h);
            },
        )
    }

    /// 确保底层对象已创建。打算在中断里 `release` 时，先在线程中调用一次。
    pub fn init(&self) {
        self.handle();
    }

    pub fn acquire(&self) {
        unsafe { sys::tls_os_sem_acquire(self.handle(), 0) };
    }

    /// 在 `timeout` 内获取到信号量时返回 `true`（最小等待 2ms）。
    pub fn acquire_timeout(&self, timeout: Duration) -> bool {
        ok(unsafe { sys::tls_os_sem_acquire(self.handle(), ticks(timeout)) })
    }

    pub fn release(&self) {
        unsafe { sys::tls_os_sem_release(self.handle()) };
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        if let Some(h) = self.raw.existing() {
            unsafe { sys::tls_os_sem_delete(h) };
        }
    }
}

/// 多生产者多消费者的有界消息队列（FreeRTOS queue）。
///
/// 不超过 4 字节的值直接存放在队列槽里，发送时不分配内存，因此可以在中断里 `send`；
/// 更大的值会先装进 `Box`。
pub struct Channel<T> {
    raw: LazyHandle,
    capacity: u32,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for Channel<T> {}
unsafe impl<T: Send> Sync for Channel<T> {}

impl<T> Channel<T> {
    const INLINE: bool =
        mem::size_of::<T>() <= mem::size_of::<*mut c_void>() && mem::align_of::<T>() <= mem::align_of::<*mut c_void>();

    pub const fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self { raw: LazyHandle::new(), capacity: capacity as u32, _marker: PhantomData }
    }

    fn handle(&self) -> *mut c_void {
        self.raw.get(
            || {
                let mut h = ptr::null_mut();
                unsafe { sys::tls_os_queue_create(&mut h, self.capacity) };
                h
            },
            |h| unsafe {
                sys::tls_os_queue_delete(h);
            },
        )
    }

    /// 确保底层对象已创建。打算在中断里 `send` 时，先在线程中调用一次。
    pub fn init(&self) {
        self.handle();
    }

    fn encode(value: T) -> *mut c_void {
        if Self::INLINE {
            let mut slot = MaybeUninit::<*mut c_void>::zeroed();
            unsafe {
                slot.as_mut_ptr().cast::<T>().write(value);
                slot.assume_init()
            }
        } else {
            Box::into_raw(Box::new(value)).cast()
        }
    }

    unsafe fn decode(raw: *mut c_void) -> T {
        if Self::INLINE {
            unsafe { (&raw const raw).cast::<T>().read() }
        } else {
            *unsafe { Box::from_raw(raw.cast::<T>()) }
        }
    }

    /// 发送一条消息，不等待；队列已满时把值原样退回。
    pub fn send(&self, value: T) -> Result<(), T> {
        let raw = Self::encode(value);
        if ok(unsafe { sys::tls_os_queue_send(self.handle(), raw, 0) }) {
            Ok(())
        } else {
            Err(unsafe { Self::decode(raw) })
        }
    }

    /// 接收一条消息，必要时一直等待。
    pub fn recv(&self) -> T {
        loop {
            if let Some(v) = self.recv_ticks(0) {
                return v;
            }
        }
    }

    /// 在 `timeout` 内接收一条消息（最小等待 2ms）。
    pub fn recv_timeout(&self, timeout: Duration) -> Option<T> {
        self.recv_ticks(ticks(timeout))
    }

    fn recv_ticks(&self, ticks: u32) -> Option<T> {
        let mut raw = ptr::null_mut();
        let status = unsafe { sys::tls_os_queue_receive(self.handle(), &mut raw, 0, ticks) };
        ok(status).then(|| unsafe { Self::decode(raw) })
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if self.raw.existing().is_none() {
            return;
        }
        if mem::needs_drop::<T>() || !Self::INLINE {
            while self.recv_ticks(1).is_some() {}
        }
        if let Some(h) = self.raw.existing() {
            unsafe { sys::tls_os_queue_delete(h) };
        }
    }
}

/// 在关中断的临界区内执行 `f`，用于和中断共享的数据。临界区应尽量短。
pub fn critical<R>(f: impl FnOnce() -> R) -> R {
    let sr = unsafe { sys::tls_os_set_critical() };
    let r = f();
    unsafe { sys::tls_os_release_critical(sr) };
    r
}
