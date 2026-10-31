//! LLVM 的 C-SKY 后端把所有原子操作编译成 `__atomic_*` 库函数调用，而工具链没有提供 libatomic。
//! W800 是单核处理器，这里在关中断的情况下用普通读写实现读-改-写操作；对齐的读、写本身就是原子的。

/// 关中断，返回之前的 PSR。
#[inline(always)]
fn irq_save() -> u32 {
    let psr: u32;
    // LLVM 的 C-SKY 汇编器不支持 psr / ie 这两种操作数，直接写机器码：mfcr r0, psr; psrclr ie
    unsafe { core::arch::asm!(".short 0xc000, 0x6020", ".short 0xc080, 0x7020", out("a0") psr, options(nostack)) };
    psr
}

#[inline(always)]
fn irq_restore(psr: u32) {
    // mtcr r0, psr
    unsafe { core::arch::asm!(".short 0xc000, 0x6420", in("a0") psr, options(nostack)) };
}

#[inline(always)]
unsafe fn rmw<T: Copy>(ptr: *mut T, f: impl FnOnce(T) -> T) -> T {
    let psr = irq_save();
    let old = unsafe { ptr.read_volatile() };
    unsafe { ptr.write_volatile(f(old)) };
    irq_restore(psr);
    old
}

/// 内存屏障（`fence`）。单核处理器上不需要硬件屏障，函数调用本身就阻止了编译器重排。
#[unsafe(no_mangle)]
extern "C" fn __sync_synchronize() {}

macro_rules! atomics {
    ($t:ty, $load:ident, $store:ident, $xchg:ident, $cas:ident,
     $add:ident, $sub:ident, $and:ident, $or:ident, $xor:ident, $nand:ident) => {
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $load(ptr: *const $t, _order: i32) -> $t {
            unsafe { ptr.read_volatile() }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $store(ptr: *mut $t, val: $t, _order: i32) {
            unsafe { ptr.write_volatile(val) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $xchg(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |_| val) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $cas(ptr: *mut $t, expected: *mut $t, desired: $t, _success: i32, _failure: i32) -> bool {
            let psr = irq_save();
            let current = unsafe { ptr.read_volatile() };
            let ok = current == unsafe { *expected };
            if ok {
                unsafe { ptr.write_volatile(desired) };
            } else {
                unsafe { *expected = current };
            }
            irq_restore(psr);
            ok
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $add(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| v.wrapping_add(val)) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $sub(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| v.wrapping_sub(val)) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $and(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| v & val) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $or(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| v | val) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $xor(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| v ^ val) }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $nand(ptr: *mut $t, val: $t, _order: i32) -> $t {
            unsafe { rmw(ptr, |v| !(v & val)) }
        }
    };
}

atomics!(u8, __atomic_load_1, __atomic_store_1, __atomic_exchange_1, __atomic_compare_exchange_1,
    __atomic_fetch_add_1, __atomic_fetch_sub_1, __atomic_fetch_and_1, __atomic_fetch_or_1,
    __atomic_fetch_xor_1, __atomic_fetch_nand_1);
atomics!(u16, __atomic_load_2, __atomic_store_2, __atomic_exchange_2, __atomic_compare_exchange_2,
    __atomic_fetch_add_2, __atomic_fetch_sub_2, __atomic_fetch_and_2, __atomic_fetch_or_2,
    __atomic_fetch_xor_2, __atomic_fetch_nand_2);
atomics!(u32, __atomic_load_4, __atomic_store_4, __atomic_exchange_4, __atomic_compare_exchange_4,
    __atomic_fetch_add_4, __atomic_fetch_sub_4, __atomic_fetch_and_4, __atomic_fetch_or_4,
    __atomic_fetch_xor_4, __atomic_fetch_nand_4);
