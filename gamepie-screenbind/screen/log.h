#ifndef __LOG_H__
#define __LOG_H__

#ifdef __cplusplus
extern "C" {
#endif

enum log_level
{
   L_DEBUG = 0,
   L_INFO,
   L_WARN,
   L_ERROR
};

typedef void (*log_printf_t)(enum log_level level,
      const char *fmt, ...);

extern log_printf_t log_printf;

#ifdef __cplusplus
}
#endif

#endif
