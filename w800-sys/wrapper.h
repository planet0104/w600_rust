/* bindgen 的入口头文件：w800-sys 暴露的全部 SDK 接口。
 * 不要包含 minilibc 的 <errno.h>：socket 返回的是 lwIP 自己的错误码（LWIP_PROVIDE_ERRNO）。 */
#include "wm_include.h"
#include "lwip/arch.h"
#include "wm_gpio_afsel.h"
#include "wm_io.h"
#include "wm_timer.h"
#include "wm_i2c.h"
#include "wm_adc.h"
#include "wm_internal_flash.h"
#include "wm_flash_map.h"
#include "wm_cpu.h"
#include "wm_watchdog.h"
#include "wm_irq.h"
#include "wm_wifi_oneshot.h"
#include "wm_bt_config.h"
#include "wm_bt_api.h"
#include "host/ble_hs.h"
#include "services/gap/ble_svc_gap.h"

/* 以下函数在 SDK 中有实现，但没有出现在头文件里。 */

/* platform/arch/xt804/libc/libc_port.c：轮询方式向调试串口（UART0）输出一个字符，'\n' 前补 '\r'。 */
int sendchar(int ch);

/* platform/common/mem/wm_mem.c：堆的剩余字节数。 */
u32 tls_mem_get_avail_heapsize(void);

/* src/app/bleapp/wm_bt_app.c：启动 NimBLE 协议栈和蓝牙控制器，阻塞到协议栈就绪。uart_idx 为 0xFF 表示不使用 HCI 串口。 */
int tls_bt_init(uint8_t uart_idx);
