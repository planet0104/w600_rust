pub use crate::print::_print;

/// `entry!` 未指定栈大小时主线程使用的栈大小（字节）。
pub const DEFAULT_MAIN_STACK_SIZE: usize = 8 * 1024;

/// 声明程序入口。
///
/// SDK 初始化完成后会调用 `UserMain`，并且要求它尽快返回（之后还有自动联网等初始化）。
/// 因此入口函数会在一个新线程里运行，可以放心地在里面写无限循环。
///
/// ```ignore
/// w600::entry!(main);                       // 主线程栈 8KB
/// w600::entry!(main, stack_size = 16384);   // 自定义栈大小
/// ```
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        $crate::entry!($main, stack_size = $crate::rt::DEFAULT_MAIN_STACK_SIZE);
    };
    ($main:path, stack_size = $size:expr) => {
        #[unsafe(no_mangle)]
        extern "C" fn UserMain() {
            let main: fn() = $main;
            $crate::rt::start(main, $size);
        }
    };
}

#[doc(hidden)]
pub fn start(main: fn(), stack_size: usize) {
    crate::thread::Builder::new()
        .name("main")
        .stack_size(stack_size)
        .spawn(main)
        .expect("failed to spawn main thread");
}

/// 打印 panic 信息后，让出错的线程永久休眠，其余线程和 Wi-Fi 继续运行；
/// 若 panic 发生在中断里则原地停机。需要其他策略（如复位）时可关闭 `panic-handler` 特性自行实现。
#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    // 不加锁：持锁的线程可能正是 panic 的线程。
    let _ = writeln!(crate::print::Stdout, "\npanic: {info}");
    loop {
        if crate::in_interrupt() {
            core::hint::spin_loop();
        } else {
            unsafe { crate::sys::tls_os_time_delay(crate::sys::configTICK_RATE_HZ) };
        }
    }
}
