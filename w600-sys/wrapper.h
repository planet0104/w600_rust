/* bindgen 的入口头文件：w600-sys 暴露的全部 SDK 接口。
 * 不要包含 newlib 的 <errno.h>：socket 返回的是 lwIP 自己的错误码（LWIP_PROVIDE_ERRNO），
 * 部分取值与 newlib 不同。 */
#include "wm_include.h"
#include "lwip/arch.h"
#include "wm_netif2.0.3.h"
#include "wm_sockets2.0.3.h"
#include "wm_gpio_afsel.h"
#include "wm_io.h"
#include "wm_timer.h"
#include "wm_i2c.h"
#include "wm_adc.h"
#include "wm_internal_flash.h"
#include "wm_flash_map.h"
#include "wm_cpu.h"
#include "wm_watchdog.h"
