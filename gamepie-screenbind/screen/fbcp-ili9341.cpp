#include <fcntl.h>
#include <linux/fb.h>
#include <linux/futex.h>
#include <linux/spi/spidev.h>
#include <memory.h>
#include <stdio.h>
#include <stdlib.h>
#include <endian.h>
#include <sys/mman.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>
#include <inttypes.h>
#include <math.h>
#include <signal.h>

#include "config.h"
#include "text.h"
#include "spi.h"
#include "gpu.h"
#include "statistics.h"
#include "tick.h"
#include "display.h"
#include "util.h"
#include "mailbox.h"
#include "diff.h"
#include "mem_alloc.h"
#include "low_battery.h"

#include "fbcp-ili9341.h"

int CountNumChangedPixels(uint16_t *framebuffer, uint16_t *prevFramebuffer)
{
  int changedPixels = 0;
  for(int y = 0; y < gpuFrameHeight; ++y)
  {
    for(int x = 0; x < gpuFrameWidth; ++x)
      if (framebuffer[x] != prevFramebuffer[x])
        ++changedPixels;

    framebuffer += gpuFramebufferScanlineStrideBytes >> 1;
    prevFramebuffer += gpuFramebufferScanlineStrideBytes >> 1;
  }
  return changedPixels;
}

uint64_t displayContentsLastChanged = 0;

volatile bool programRunning = true;
volatile bool initialised = false;

const char *SignalToString(int signal)
{
  if (signal == SIGINT) return "SIGINT";
  if (signal == SIGQUIT) return "SIGQUIT";
  if (signal == SIGUSR1) return "SIGUSR1";
  if (signal == SIGUSR2) return "SIGUSR2";
  if (signal == SIGTERM) return "SIGTERM";
  return "?";
}

int spiX;
int spiY;
int spiEndX;
int size;
uint16_t *framebuffer[2];
uint32_t curFrameEnd;
uint32_t prevFrameEnd;
bool prevFrameWasInterlacedUpdate;
bool interlacedUpdate;
int frameParity;
log_printf_t log_printf;

uint16_t lcd_lib_width()
{
  return DISPLAY_NATIVE_WIDTH;
}
uint16_t lcd_lib_height()
{
  return DISPLAY_NATIVE_HEIGHT;
}

void lcd_lib_init(log_printf_t log_func)
{
  log_printf = log_func;
  log_printf(L_INFO, "LCD Driver starting\n");
  OpenMailbox();
  InitSPI();
  displayContentsLastChanged = tick();
  InitLowBatterySystem();

  // Track current SPI display controller write X and Y cursors.
  spiX = -1;
  spiY = -1;
  spiEndX = DISPLAY_WIDTH;

  InitGPU();

  spans = (Span*)Malloc((gpuFrameWidth * gpuFrameHeight / 2) * sizeof(Span), "main() task spans");
  size = gpuFramebufferSizeBytes;
  framebuffer[0] = (uint16_t *)Malloc(size, "main() framebuffer0");
  framebuffer[1] = (uint16_t *)Malloc(size, "main() framebuffer1");
  memset(framebuffer[0], 0, size); // Doublebuffer received GPU memory contents, first buffer contains current GPU memory,
  memset(framebuffer[1], 0, size); // second buffer contains whatever the display is currently showing. This allows diffing pixels between the two.

  curFrameEnd = spiTaskMemory->queueTail;
  prevFrameEnd = spiTaskMemory->queueTail;

  prevFrameWasInterlacedUpdate = false;
  interlacedUpdate = false; // True if the previous update we did was an interlaced half field update.
  frameParity = 0; // For interlaced frame updates, this is either 0 or 1 to denote evens or odds.
  log_printf(L_DEBUG, "All initialised, now running main loop...\n");
  initialised = true;
}

void lcd_lib_tick(const uint16_t *data, int force_full)
{
  if (!initialised)
  {
    log_printf(L_WARN, "LCD not initialised before trying to draw\n");
    return;
  }

  // No dealing with waiting for frames on interlacing when pushing frames.
  prevFrameWasInterlacedUpdate = interlacedUpdate;

  // At all times keep at most two rendered frames in the SPI task queue pending to be displayed. Only proceed to submit a new frame
  // once the older of those has been displayed.
  if ((spiTaskMemory->queueTail + SPI_QUEUE_SIZE - spiTaskMemory->queueHead) % SPI_QUEUE_SIZE > (spiTaskMemory->queueTail + SPI_QUEUE_SIZE - prevFrameEnd) % SPI_QUEUE_SIZE)
  {
    double usecsUntilSpiQueueEmpty = spiTaskMemory->spiBytesQueued*spiUsecsPerByte;
    if (usecsUntilSpiQueueEmpty > 0)
    {
      uint32_t sleepUsecs = (uint32_t)(usecsUntilSpiQueueEmpty*0.4);
      if (sleepUsecs > 1000)
      {
        log_printf(L_WARN, "Potentially too much work in SPI task queue");
      }
    }
  }

  int expiredFrames = 0;
  uint64_t now = tick();
  while(expiredFrames < frameTimeHistorySize && now - frameTimeHistory[expiredFrames].time >= FRAMERATE_HISTORY_LENGTH) ++expiredFrames;
  if (expiredFrames > 0)
  {
    frameTimeHistorySize -= expiredFrames;
    for(int i = 0; i < frameTimeHistorySize; ++i) frameTimeHistory[i] = frameTimeHistory[i+expiredFrames];
  }

#ifdef STATISTICS
  int expiredSkippedFrames = 0;
  while(expiredSkippedFrames < frameSkipTimeHistorySize && now - frameSkipTimeHistory[expiredSkippedFrames] >= 1000000/*FRAMERATE_HISTORY_LENGTH*/) ++expiredSkippedFrames;
  if (expiredSkippedFrames > 0)
  {
    frameSkipTimeHistorySize -= expiredSkippedFrames;
    for(int i = 0; i < frameSkipTimeHistorySize; ++i) frameSkipTimeHistory[i] = frameSkipTimeHistory[i+expiredSkippedFrames];
  }
#endif

  bool framebufferHasNewChangedPixels = true;
  uint64_t frameObtainedTime;

  frameObtainedTime = tick();
  uint64_t framePollingStartTime = frameObtainedTime;

  memcpy(framebuffer[0], data, gpuFrameWidth*gpuFrameHeight*2);

  PollLowBattery();

  DrawStatisticsOverlay(framebuffer[0]);
  DrawLowBatteryIcon(framebuffer[0]);

  framebufferHasNewChangedPixels = true;

  // If too many pixels have changed on screen, drop adaptively to interlaced updating to keep up the frame rate.
  double inputDataFps = 120; // TODO how to get frame rate
  double desiredTargetFps = MAX(1, MIN(inputDataFps, TARGET_FRAME_RATE));
#ifdef SINGLE_CORE_BOARD
  const double timesliceToUseForScreenUpdates = 250000;
#elif defined(ILI9486) || defined(ILI9486L) ||defined(HX8357D)
  const double timesliceToUseForScreenUpdates = 750000;
#else
  const double timesliceToUseForScreenUpdates = 1500000;
#endif
  const double tooMuchToUpdateUsecs = timesliceToUseForScreenUpdates / desiredTargetFps; // If updating the current and new frame takes too many frames worth of allotted time, drop to interlacing.

#if !defined(NO_INTERLACING) || (defined(BACKLIGHT_CONTROL) && defined(TURN_DISPLAY_OFF_AFTER_USECS_OF_INACTIVITY))
  int numChangedPixels = framebufferHasNewChangedPixels ? CountNumChangedPixels(framebuffer[0], framebuffer[1]) : 0;
#endif

#ifdef NO_INTERLACING
  interlacedUpdate = false;
#elif defined(ALWAYS_INTERLACING)
  interlacedUpdate = (numChangedPixels > 0);
#else
  uint32_t bytesToSend = numChangedPixels * SPI_BYTESPERPIXEL + (DISPLAY_DRAWABLE_HEIGHT<<1);
  interlacedUpdate = ((bytesToSend + spiTaskMemory->spiBytesQueued) * spiUsecsPerByte > tooMuchToUpdateUsecs); // Decide whether to do interlacedUpdate - only updates half of the screen
#endif
  if (force_full) interlacedUpdate = false;

  if (interlacedUpdate) frameParity = 1-frameParity; // Swap even-odd fields every second time we do an interlaced update (progressive updates ignore field order)
  int bytesTransferred = 0;
  Span *head = 0;

#if defined(ALL_TASKS_SHOULD_DMA) && defined(UPDATE_FRAMES_WITHOUT_DIFFING)
  NoDiffChangedRectangle(head);
#elif defined(ALL_TASKS_SHOULD_DMA) && defined(UPDATE_FRAMES_IN_SINGLE_RECTANGULAR_DIFF)
  DiffFramebuffersToSingleChangedRectangle(framebuffer[0], framebuffer[1], head);
#else
  // Collect all spans in this image
  if (framebufferHasNewChangedPixels || prevFrameWasInterlacedUpdate)
  {
    // If possible, utilize a faster 4-wide pixel diffing method
#ifdef FAST_BUT_COARSE_PIXEL_DIFF
    if (gpuFrameWidth % 4 == 0 && gpuFramebufferScanlineStrideBytes % 8 == 0)
      DiffFramebuffersToScanlineSpansFastAndCoarse4Wide(framebuffer[0], framebuffer[1], interlacedUpdate, frameParity, head);
    else
#endif
      DiffFramebuffersToScanlineSpansExact(framebuffer[0], framebuffer[1], interlacedUpdate, frameParity, head); // If disabled, or framebuffer width is not compatible, use the exact method
  }

  // Merge spans together on adjacent scanlines - works only if doing a progressive update
  if (!interlacedUpdate)
    MergeScanlineSpanList(head);
#endif

// TODO VSYNC start
  if (head) // do we have a new frame?
  {
    // If using vsync, this main thread is responsible for maintaining the frame histogram. If not using vsync,
    // but instead are using a dedicated GPU thread, then that dedicated thread maintains the frame histogram,
    // in which case this is not needed.
    AddHistogramSample(frameObtainedTime);

    // We got a new frame, so update contents of the statistics overlay as well
    RefreshStatisticsOverlayText();
  }
// VSYNC end

  // Submit spans
  for(Span *i = head; i; i = i->next)
  {
#ifdef ALIGN_TASKS_FOR_DMA_TRANSFERS
    // DMA transfers smaller than 4 bytes are causing trouble, so in order to ensure smooth DMA operation,
    // make sure each message is at least 4 bytes in size, hence one pixel spans are forbidden:
    if (i->size == 1)
    {
      if (i->endX < DISPLAY_DRAWABLE_WIDTH) { ++i->endX; ++i->lastScanEndX; }
      else --i->x;
      ++i->size;
    }
#endif
    // Update the write cursor if needed
#ifndef DISPLAY_WRITE_PIXELS_CMD_DOES_NOT_RESET_WRITE_CURSOR
    if (spiY != i->y)
#endif
    {
#if defined(MUST_SEND_FULL_CURSOR_WINDOW) || defined(ALIGN_TASKS_FOR_DMA_TRANSFERS)
      QUEUE_SET_WRITE_WINDOW_TASK(DISPLAY_SET_CURSOR_Y, displayYOffset + i->y, displayYOffset + gpuFrameHeight - 1);
#else
      QUEUE_MOVE_CURSOR_TASK(DISPLAY_SET_CURSOR_Y, displayYOffset + i->y);
#endif
      IN_SINGLE_THREADED_MODE_RUN_TASK();
      spiY = i->y;
    }

    if (i->endY > i->y + 1 && (spiX != i->x || spiEndX != i->endX)) // Multiline span?
    {
      QUEUE_SET_WRITE_WINDOW_TASK(DISPLAY_SET_CURSOR_X, displayXOffset + i->x, displayXOffset + i->endX - 1);
      IN_SINGLE_THREADED_MODE_RUN_TASK();
      spiX = i->x;
      spiEndX = i->endX;
    }
    else // Singleline span
    {
#ifdef ALIGN_TASKS_FOR_DMA_TRANSFERS
      if (spiX != i->x || spiEndX < i->endX)
      {
        QUEUE_SET_WRITE_WINDOW_TASK(DISPLAY_SET_CURSOR_X, displayXOffset + i->x, displayXOffset + gpuFrameWidth - 1);
        IN_SINGLE_THREADED_MODE_RUN_TASK();
        spiX = i->x;
        spiEndX = gpuFrameWidth;
      }
#else
      if (spiEndX < i->endX) // Need to push the X end window?
      {
        // We are doing a single line span and need to increase the X window. If possible,
        // peek ahead to cater to the next multiline span update if that will be compatible.
        int nextEndX = gpuFrameWidth;
        for(Span *j = i->next; j; j = j->next)
          if (j->endY > j->y+1)
          {
            if (j->endX >= i->endX) nextEndX = j->endX;
            break;
          }
        QUEUE_SET_WRITE_WINDOW_TASK(DISPLAY_SET_CURSOR_X, displayXOffset + i->x, displayXOffset + nextEndX - 1);
        IN_SINGLE_THREADED_MODE_RUN_TASK();
        spiX = i->x;
        spiEndX = nextEndX;
      }
      else
#ifndef DISPLAY_WRITE_PIXELS_CMD_DOES_NOT_RESET_WRITE_CURSOR
      if (spiX != i->x)
#endif
      {
#ifdef MUST_SEND_FULL_CURSOR_WINDOW
        QUEUE_SET_WRITE_WINDOW_TASK(DISPLAY_SET_CURSOR_X, displayXOffset + i->x, displayXOffset + spiEndX - 1);
#else
        QUEUE_MOVE_CURSOR_TASK(DISPLAY_SET_CURSOR_X, displayXOffset + i->x);
#endif
        IN_SINGLE_THREADED_MODE_RUN_TASK();
        spiX = i->x;
      }
#endif
    }

    // Submit the span pixels
    SPITask *task = AllocTask(i->size*SPI_BYTESPERPIXEL);
    task->cmd = DISPLAY_WRITE_PIXELS;

    bytesTransferred += task->PayloadSize()+1;
    uint16_t *scanline = framebuffer[0] + i->y * (gpuFramebufferScanlineStrideBytes>>1);
    uint16_t *prevScanline = framebuffer[1] + i->y * (gpuFramebufferScanlineStrideBytes>>1);

#ifdef OFFLOAD_PIXEL_COPY_TO_DMA_CPP
    // If running a singlethreaded build without a separate SPI thread, we can offload the whole flow of the pixel data out to the code in the dma.cpp backend,
    // which does the pixel task handoff out to DMA in inline assembly. This is done mainly to save an extra memcpy() when passing data off from GPU to SPI,
    // since in singlethreaded mode, snapshotting GPU and sending data to SPI is done sequentially in this main loop.
    // In multithreaded builds, this approach cannot be used, since after we snapshot a frame, we need to send it off to SPI thread to process, and make a copy
    // anways to ensure it does not get overwritten.
    task->fb = (uint8_t*)(scanline + i->x);
    task->prevFb = (uint8_t*)(prevScanline + i->x);
    task->width = i->endX - i->x;
#else
    uint16_t *data = (uint16_t*)task->data;
    for(int y = i->y; y < i->endY; ++y, scanline += gpuFramebufferScanlineStrideBytes>>1, prevScanline += gpuFramebufferScanlineStrideBytes>>1)
    {
      int endX = (y + 1 == i->endY) ? i->lastScanEndX : i->endX;
      int x = i->x;
#ifdef DISPLAY_COLOR_FORMAT_R6X2G6X2B6X2
      // Convert from R5G6B5 to R6X2G6X2B6X2 on the fly
      while(x < endX)
      {
        uint16_t pixel = scanline[x++];
        uint16_t r = (pixel >> 8) & 0xF8;
        uint16_t g = (pixel >> 3) & 0xFC;
        uint16_t b = (pixel << 3) & 0xF8;
        ((uint8_t*)data)[0] = r | (r >> 5); // On red and blue color channels, need to expand 5 bits to 6 bits. Do that by duplicating the highest bit as lowest bit.
        ((uint8_t*)data)[1] = g;
        ((uint8_t*)data)[2] = b | (b >> 5);
        data = (uint16_t*)((uintptr_t)data + 3);
      }
#else
      while(x < endX && (x&1)) *data++ = __builtin_bswap16(scanline[x++]);
      while(x < (endX&~1U))
      {
        uint32_t u = *(uint32_t*)(scanline+x);
        *(uint32_t*)data = ((u & 0xFF00FF00U) >> 8) | ((u & 0x00FF00FFU) << 8);
        data += 2;
        x += 2;
      }
      while(x < endX) *data++ = __builtin_bswap16(scanline[x++]);
#endif
#if !(defined(ALL_TASKS_SHOULD_DMA) && defined(UPDATE_FRAMES_WITHOUT_DIFFING)) // If not diffing, no need to maintain prev frame.
      memcpy(prevScanline+i->x, scanline+i->x, (endX - i->x)*FRAMEBUFFER_BYTESPERPIXEL);
#endif
    }
#endif
    CommitTask(task);
    IN_SINGLE_THREADED_MODE_RUN_TASK();
  }

#ifdef KERNEL_MODULE_CLIENT
  // Wake the kernel module up to run tasks. TODO: This might not be best placed here, we could pre-empt
  // to start running tasks already half-way during task submission above.
  if (spiTaskMemory->queueHead != spiTaskMemory->queueTail && !(spi->cs & BCM2835_SPI0_CS_TA))
    spi->cs |= BCM2835_SPI0_CS_TA;
#endif

  // Remember where in the command queue this frame ends, to keep track of the SPI thread's progress over it
  if (bytesTransferred > 0)
  {
    prevFrameEnd = curFrameEnd;
    curFrameEnd = spiTaskMemory->queueTail;
  }

#ifdef STATISTICS
  if (bytesTransferred > 0)
  {
    if (frameTimeHistorySize < FRAME_HISTORY_MAX_SIZE)
    {
      frameTimeHistory[frameTimeHistorySize].interlaced = interlacedUpdate || prevFrameWasInterlacedUpdate;
      frameTimeHistory[frameTimeHistorySize++].time = tick();
    }
    AddFrameCompletionTimeMarker();
  }
  statsBytesTransferred += bytesTransferred;
#endif
}

void lcd_lib_deinit()
{
  programRunning = false;
  initialised = false;
  DeinitGPU();
  DeinitSPI();
  CloseMailbox();
  log_printf(L_INFO, "LCD Driver quitting\n");
}
