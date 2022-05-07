#include "cshim.h"
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>

void gamepie_log(unsigned, const char *msg);

void vgamepie_log_shim(unsigned level, const char *fmt, va_list argp)
{
    size_t needed = vsnprintf(NULL, 0, fmt, argp)+1;
    char *buffer = malloc(needed);
    vsprintf(buffer, fmt, argp);
    gamepie_log(level, buffer);
    free(buffer);
}

void gamepie_log_shim(unsigned level, const char *fmt, ...) {
    va_list argp;
    va_start(argp, fmt);
    vgamepie_log_shim(level, fmt, argp);
    va_end(argp);
}