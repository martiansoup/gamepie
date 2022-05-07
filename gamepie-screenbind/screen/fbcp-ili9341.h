#ifndef __FBCP_ILI9341_H__
#define __FBCP_ILI9341_H__

#include <stdint.h>
#include "log.h"

#ifdef __cplusplus
extern "C" {
#endif

uint16_t lcd_lib_width();
uint16_t lcd_lib_height();
void lcd_lib_init(log_printf_t log_func);
void lcd_lib_tick(const uint16_t *data, int force_full);
void lcd_lib_deinit();

#ifdef __cplusplus
}
#endif

#endif
