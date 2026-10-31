use core::alloc::{GlobalAlloc, Layout};

use crate::sys;

/// `mem_alloc_debug` 在 newlib `malloc` 返回的 8 字节对齐地址后加了 8 字节头部，结果仍是 8 字节对齐。
const MIN_ALIGN: usize = 8;

/// 全局分配器直接使用 SDK 的堆（`mem_alloc_debug`）：它对 newlib `malloc` 加了锁，
/// 在中断里则自动改用 FreeRTOS 的 heap_2。Rust 与 C 代码共享同一个堆，不需要另外划分内存。
///
/// 不覆盖 `realloc`：SDK 的 `mem_realloc_debug` 在中断分支里有 bug，使用默认的“分配-拷贝-释放”。
struct SdkHeap;

unsafe impl GlobalAlloc for SdkHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN {
            return unsafe { sys::mem_alloc_debug(layout.size() as u32) }.cast();
        }
        // 超出 8 字节的对齐：多分配 align 字节，把原始指针存在对齐地址的前一个字里。
        let raw: *mut u8 = unsafe { sys::mem_alloc_debug((layout.size() + layout.align()) as u32) }.cast();
        if raw.is_null() {
            return raw;
        }
        let offset = layout.align() - (raw as usize & (layout.align() - 1));
        unsafe {
            let ptr = raw.add(offset);
            ptr.cast::<*mut u8>().sub(1).write(raw);
            ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let raw = if layout.align() <= MIN_ALIGN {
            ptr
        } else {
            unsafe { ptr.cast::<*mut u8>().sub(1).read() }
        };
        unsafe { sys::mem_free_debug(raw.cast()) }
    }
}

#[global_allocator]
static HEAP: SdkHeap = SdkHeap;
