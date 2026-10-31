/* W600 链接脚本，基于官方 SDK 的 Tools/GNU/link_w600_1m.ld。
 * 使用方式：在应用的 .cargo/config.toml 中加入 "-C", "link-arg=-Tw600.x"。
 * 默认配合 rust-lld 使用，也兼容 GNU ld（arm-none-eabi-gcc 加 -nostdlib）。
 *
 * 1MB Flash 布局：0x8000000 起为 secboot，应用镜像头位于 0x8010000，代码从 0x8010100 开始，
 * 0x8090000 起为 OTA 升级区，因此应用最大 0x7FF00 字节。
 */
MEMORY
{
  FLASH (rx) : ORIGIN = 0x8010100, LENGTH = 0x7FF00
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 0x38000
}

/* 必须写在 GROUP 之前：先拉入 SDK 的 retarget_gcc.o，
 * 否则 newlib 会先用自己的 _write_r/_sbrk_r 桩函数，造成重复定义。 */
EXTERN(_write_r _sbrk_r)
EXTERN(Reset_Handler)

/* SDK 各库之间、SDK 与 newlib 之间互相引用，放进同一个 GROUP 反复解析。
 * libc / libm / libgcc 是随 w600-sys 提供的 newlib（thumb/v7-m/nofp），不需要安装 ARM 工具链。 */
GROUP(-lw600sdk -lwlan -lairkiss_log -lc -lm -lgcc)
ENTRY(Reset_Handler)

SECTIONS
{
	.text :
	{
		KEEP(*(.isr_vector))
		*(.text*)

		KEEP(*(.init))
		KEEP(*(.fini))

		*crtbegin.o(.ctors)
		*crtbegin?.o(.ctors)
		*(EXCLUDE_FILE(*crtend?.o *crtend.o) .ctors)
		*(SORT(.ctors.*))
		*(.ctors)

		*crtbegin.o(.dtors)
		*crtbegin?.o(.dtors)
		*(EXCLUDE_FILE(*crtend?.o *crtend.o) .dtors)
		*(SORT(.dtors.*))
		*(.dtors)

		*(.rodata*)

		KEEP(*(.eh_frame*))
	} > FLASH

	.ARM.extab :
	{
		*(.ARM.extab* .gnu.linkonce.armextab.*)
	} > FLASH

	__exidx_start = .;
	.ARM.exidx :
	{
		*(.ARM.exidx* .gnu.linkonce.armexidx.*)
	} > FLASH
	__exidx_end = .;

	__etext = .;

	.data : AT (__etext)
	{
		__data_start__ = .;
		*(vtable)
		*(.data*)

		. = ALIGN(4);
		PROVIDE_HIDDEN (__preinit_array_start = .);
		KEEP(*(.preinit_array))
		PROVIDE_HIDDEN (__preinit_array_end = .);

		. = ALIGN(4);
		PROVIDE_HIDDEN (__init_array_start = .);
		KEEP(*(SORT(.init_array.*)))
		KEEP(*(.init_array))
		PROVIDE_HIDDEN (__init_array_end = .);

		. = ALIGN(4);
		PROVIDE_HIDDEN (__fini_array_start = .);
		KEEP(*(SORT(.fini_array.*)))
		KEEP(*(.fini_array))
		PROVIDE_HIDDEN (__fini_array_end = .);

		KEEP(*(.jcr*))
		. = ALIGN(4);
		__data_end__ = .;
	} > RAM

	/* 启动阶段和中断使用的主栈，大小与启动文件的 Stack_Size（0xC00）一致。
	 * 启动文件里的 .stack/.heap 段没有 "a" 标志，rust-lld 不会为它们分配地址
	 * （GNU ld 原版脚本靠 (COPY) 绕过），所以在这里预留空间并定义相关符号，丢弃原段。 */
	.stack (NOLOAD) :
	{
		. = ALIGN(8);
		__stack_start__ = .;
		. += 0xC00;
		__stack_end__ = .;
	} > RAM
	__StackLimit = __stack_start__;
	__StackTop = __stack_end__;

	.bss (NOLOAD) :
	{
		. = ALIGN(4);
		__bss_start__ = .;
		*(.bss*)
		*(COMMON)
		. = ALIGN(4);
		__bss_end__ = .;
	} > RAM

	/* 堆从 end 一直用到 RAM 末尾（0x20038000）。
	 * 0x20028000 以上原本只有 ADC 的 DMA 模式会使用，Wi-Fi 缓冲区位于 0x2003C000。 */
	. = ALIGN(8);
	__end__ = .;
	PROVIDE(end = .);
	__heap_base = .;
	__HeapLimit = ORIGIN(RAM) + LENGTH(RAM);

	/DISCARD/ :
	{
		*(.stack*)
		*(.heap*)
	}

	/* SDK 要求所有任务栈都位于 0x20028000 以下。 */
	ASSERT(__StackTop <= 0x20028000, "stack address error")
	ASSERT(__HeapLimit - end >= 0x8000, "static data too large: less than 32KB left for heap")
}
