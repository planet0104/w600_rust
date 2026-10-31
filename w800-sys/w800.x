/* W800 链接脚本，基于官方 SDK 的 ld/w800/gcc_csky.ld（Copyright (C) 2017 C-SKY Microsystems, Apache-2.0）。
 * 使用方式：在应用的 .cargo/config.toml 中加入 "-C", "link-arg=-Tw800.x"，链接器为 csky-elfabiv2-gcc。
 *
 * 2MB Flash 布局：0x8000000 起为 secboot，0x8010000 起为 OTA 升级区，应用镜像头位于 0x80D0000，
 * 代码从 0x80D0400 开始，0x81E0000 起为用户数据区和系统参数区，因此应用最大 0x10FC00 字节。
 */
MEMORY
{
	FLASH  (rx)  : ORIGIN = 0x080D0400, LENGTH = 0x10FC00
	D-SRAM (rwx) : ORIGIN = 0x20000100, LENGTH = 0x47EFC
	V-SRAM (rwx) : ORIGIN = 0x20000000, LENGTH = 0x100
}

/* SDK 库由 w800-sys 以 --whole-archive 方式链接；这里只补上工具链自带的 minilibc。 */
GROUP(-lc -lm -lgcc)

/* 至少留给堆的大小。堆从 BSS 末尾一直到 __ram_end，0x2003A000 以上是 Wi-Fi 收发缓冲区。 */
__min_heap_size = 0x18000;
PROVIDE (__ram_end  = 0x2003A000);
PROVIDE (__heap_end = __ram_end);

ENTRY(Reset_Handler)
SECTIONS
{
 .text : {
  . = ALIGN(0x4) ;
  KEEP(*startup.o(.vectors))
  __stext = . ;
  *(.text)
  *(.text*)
  *(.text.*)
  *(.gnu.warning)
  *(.stub)
  *(.gnu.linkonce.t*)
  *(.jcr)
  KEEP (*(.init))
  KEEP (*(.fini))
  . = ALIGN (4) ;
  PROVIDE(__ctbp = .);
  *(.call_table_data)
  *(.call_table_text)
  . = ALIGN(0x10) ;
  __etext = . ;
 } > FLASH
 .rodata : {
  . = ALIGN(0x4) ;
  __srodata = .;
  *(.rdata)
  *(.rdata*)
  *(.rdata1)
  *(.rdata.*)
  *(.rodata)
  *(.rodata1)
  *(.rodata*)
  *(.rodata.*)
  *(.rodata.str1.4)
  . = ALIGN(0x4) ;
  __ctor_start__ = .;
  KEEP (*crtbegin.o(.ctors))
  KEEP (*crtbegin?.o(.ctors))
  KEEP (*(EXCLUDE_FILE (*crtend.o *crtend?.o ) .ctors))
  KEEP (*(SORT(.ctors.*)))
  KEEP (*(.ctors))
  __ctor_end__ = .;
  KEEP (*crtbegin.o(.dtors))
  KEEP (*crtbegin?.o(.dtors))
  KEEP (*(EXCLUDE_FILE (*crtend.o *crtend?.o ) .dtors))
  KEEP (*(SORT(.dtors.*)))
  KEEP (*(.dtors))
  __dtor_end__ = .;
  . = ALIGN(0x4) ;
  PROVIDE_HIDDEN (__preinit_array_start = .);
  KEEP (*(.preinit_array))
  PROVIDE_HIDDEN (__preinit_array_end = .);
  PROVIDE_HIDDEN (__init_array_start = .);
  KEEP (*(SORT(.init_array.*)))
  KEEP (*(.init_array))
  PROVIDE_HIDDEN (__init_array_end = .);
  PROVIDE_HIDDEN (__fini_array_start = .);
  KEEP (*(.fini_array))
  KEEP (*(SORT(.fini_array.*)))
  PROVIDE_HIDDEN (__fini_array_end = .);
  . = ALIGN(0x4) ;
  __erodata = .;
 } > FLASH
 /* 中断向量表在 RAM 中的副本，由启动代码从 __Vectors 复制。 */
 .vdata : {
  . = ALIGN(0x4) ;
  __vdata_start__ = . ;
  KEEP(*startup.o(.vdata))
  . = ALIGN(0x4) ;
  __vdata_end__ = .;
 } > V-SRAM
 /* 启动代码从 __erodata 处把初始值复制过来。 */
 .data : {
  . = ALIGN(0x4) ;
  __sdata = . ;
  __data_start__ = . ;
  data_start = . ;
  *(.got.plt)
  *(.got)
  *(.gnu.linkonce.r*)
  *(.data)
  *(.data*)
  *(.data1)
  *(.data.*)
  *(.gnu.linkonce.d*)
  *(.data1)
  *(.gcc_except_table)
  *(.gcc_except_table*)
  __start_init_call = .;
  *(.initcall.init)
  __stop_init_call = .;
  __start_cmd = .;
  *(.bootloaddata.cmd)
  . = ALIGN(4) ;
  __stop_cmd = .;
  *(.sdata)
  *(.sdata.*)
  *(.gnu.linkonce.s.*)
  *(__libc_atexit)
  *(__libc_subinit)
  *(__libc_subfreeres)
  *(.note.ABI-tag)
  . = ALIGN(0x4) ;
  __edata = .;
  __data_end__ = .;
 } > D-SRAM AT > FLASH
 .bss : {
  . = ALIGN(0x4) ;
  __sbss = ALIGN(0x4) ;
  __bss_start__ = . ;
  *(.dynsbss)
  *(.sbss)
  *(.sbss.*)
  *(.scommon)
  *(.dynbss)
  *(.bss)
  *(.bss.*)
  *(COMMON)
  . = ALIGN(0x4) ;
  __ebss = . ;
  __end = . ;
  end = . ;
  __bss_end__ = .;
 } > D-SRAM
 ._user_heap (NOLOAD) : {
  . = ALIGN(0x4) ;
  __heap_start = .;
  . += __min_heap_size;
  . = ALIGN(0x4) ;
 } > D-SRAM

 /* 不支持栈展开（panic = "abort"），丢弃展开表。 */
 /DISCARD/ : {
  *(.eh_frame) *(.eh_frame_hdr)
  *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*)
 }

 ASSERT(__heap_start + __min_heap_size <= __heap_end, "static data too large: less than 96KB left for heap")
}
