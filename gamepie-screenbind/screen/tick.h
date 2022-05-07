#pragma once

#ifndef KERNEL_MODULE
#include <inttypes.h>
#include <unistd.h>

// Initialized in spi.cpp along with the rest of the BCM2835 peripheral:
extern volatile uint32_t *systemTimerRegisterLo;
extern volatile uint32_t *systemTimerRegisterHi;
#define tick() (((uint64_t)(*systemTimerRegisterLo))|(((uint64_t)(*systemTimerRegisterHi))<<32))

#endif


#ifdef NO_THROTTLING
#define usleep(x) ((void)0)
#endif
