#include <linux/futex.h> // FUTEX_WAKE
#include <sys/syscall.h> // SYS_futex
#include <syslog.h> // syslog, LOG_ERR
#include <stdio.h> // fprintf
#include <math.h> // floor
#include <cstring>

#include "config.h"
#include "gpu.h"
#include "display.h"
#include "tick.h"
#include "util.h"
#include "statistics.h"
#include "mem_alloc.h"
#include "log.h"

bool MarkProgramQuitting(void);

// Uncomment these build options to make the display output a random performance test pattern instead of the actual
// display content. Used to debug/measure performance.
#define RANDOM_TEST_PATTERN

#define RANDOM_TEST_PATTERN_STRIPE_WIDTH DISPLAY_DRAWABLE_WIDTH

#define RANDOM_TEST_PATTERN_FRAME_RATE 60

int frameTimeHistorySize = 0;

FrameHistory frameTimeHistory[FRAME_HISTORY_MAX_SIZE] = {};

int displayXOffset = 0;
int displayYOffset = 0;
int gpuFrameWidth = 0;
int gpuFrameHeight = 0;
int gpuFramebufferScanlineStrideBytes = 0;
int gpuFramebufferSizeBytes = 0;

int excessPixelsLeft = 0;
int excessPixelsRight = 0;
int excessPixelsTop = 0;
int excessPixelsBottom = 0;

// If one first runs content that updates at e.g. 24fps, a video perhaps, the frame rate histogram will lock to that update
// rate and frame snapshots are done at 24fps. Later when user quits watching the video, and returns to e.g. 60fps updated
// launcher menu, there needs to be some mechanism that detects that update rate has now increased, and synchronizes to the
// new update rate. If snapshots keep occurring at fixed 24fps, the increase in content update rate would go unnoticed.
// Therefore maintain a "linear increases/geometric slowdowns" style of factor that pulls the frame snapshotting mechanism
// to drive itself at faster rates, poking snapshots to be performed more often to discover if the content update rate is
// more than what is currently expected.
int eagerFastTrackToSnapshottingFramesEarlierFactor = 0;

uint64_t lastFramePollTime = 0;

int RoundUpToMultipleOf(int val, int multiple)
{
  return ((val + multiple - 1) / multiple) * multiple;
}

bool SnapshotFramebuffer(uint16_t *destination)
{
  lastFramePollTime = tick();

#ifdef RANDOM_TEST_PATTERN
  // Generate random noise that updates each frame
  // uint32_t randomColor = rand() % 65536;
  static int col = 0;
  static int barY = 0;
  static uint64_t lastTestImage = tick();
  uint32_t randomColor = ((31 + ABS(col - 32)) << 5);
  uint64_t now = tick();
  if (now - lastTestImage >= 1000000/RANDOM_TEST_PATTERN_FRAME_RATE)
  {
    col = (col + 2) & 31;
    lastTestImage = now;
  }
  randomColor = randomColor | (randomColor << 16);
  uint16_t *newfb = (uint16_t*)destination;
  for(int y = 0; y < gpuFrameHeight; ++y)
  {
    int x = 0;
    const int XX = RANDOM_TEST_PATTERN_STRIPE_WIDTH;
    while(x <= gpuFrameWidth<<1)
    {
      for(int X = 0; x+X < gpuFrameWidth<<1; ++X)
      {
        if (y == barY || x+X == barY || y/2 == barY)
          newfb[x+X] = 0xFFFF;
        else if (y == barY+1 || y == barY-1)
          newfb[x+X] = 0;
        else
	  newfb[x+X] = 0xcafe;
        //  newfb[x+X] = randomColor;
      }
      x += XX + 6;
    }
    newfb += gpuFramebufferScanlineStrideBytes>>1;
  }
  barY = (barY + 1) % gpuFrameHeight;
#else
#error "No drawing code"
#endif
  return true;
}

// Since we are polling for received GPU frames, run a histogram to predict when the next frame will arrive.
// The histogram needs to be sufficiently small as to not cause a lag when frame rate suddenly changes on e.g.
// main menu <-> ingame transitions
uint64_t frameArrivalTimes[HISTOGRAM_SIZE];
uint64_t frameArrivalTimesTail = 0;
int histogramSize = 0;

// If framerate has been high for a long time, but then drops to e.g. 1fps, it would take a very very long time to fill up
// the histogram of these 1fps intervals, so fbcp-ili9341 would take a long time to go back to sleep. Introduce a max age
// for histogram entries of 10 seconds, so that if refresh rate drops from 60hz to 1hz, then after 10 seconds the histogram
// buffer will have only these 1fps intervals in it, and it will go to sleep to yield CPU time.
#define HISTOGRAM_MAX_SAMPLE_AGE 10000000

void AddHistogramSample(uint64_t t)
{
  frameArrivalTimes[frameArrivalTimesTail] = t;
  frameArrivalTimesTail = (frameArrivalTimesTail + 1) % HISTOGRAM_SIZE;
  if (histogramSize < HISTOGRAM_SIZE) ++histogramSize;

  // Expire too old entries.
  while(t - GET_HISTOGRAM(histogramSize-1) > HISTOGRAM_MAX_SAMPLE_AGE) --histogramSize;
}

int cmp(const void *e1, const void *e2) { return *(uint64_t*)e1 > *(uint64_t*)e2; }

void InitGPU()
{
  int width = DISPLAY_NATIVE_WIDTH;
  int height = DISPLAY_NATIVE_HEIGHT;
  // We may need to scale the main framebuffer to fit the native pixel size of the display. Always want to do such scaling in aspect ratio fixed mode to not stretch the image.
  // (For non-square pixels or similar, could apply a correction factor here to fix aspect ratio)

  // Often it happens that the content that is being rendered already has black letterboxes/pillarboxes if it was produced for a different aspect ratio than
  // what the current HDMI resolution is. However the current HDMI resolution might not be in the same aspect ratio as DISPLAY_DRAWABLE_WIDTH x DISPLAY_DRAWABLE_HEIGHT.
  // Therefore we may be aspect ratio correcting content that has already letterboxes/pillarboxes on it, which can result in letterboxes-on-pillarboxes, or vice versa.

  // To enable removing the double aspect ratio correction, the following settings enable "overscan": crop left/right and top/down parts of the source image
  // to remove the letterboxed parts of the source. This overscan method can also used to crop excess edges of old emulator based games intended for analog TVs,
  // e.g. NES games often had graphical artifacts on left or right edge of the screen when the game scrolls, which usually were hidden on analog TVs with overscan.

  /* In /opt/retropie/configs/nes/retroarch.cfg, if running fceumm NES emulator, put:
      aspect_ratio_index = "22"
      custom_viewport_width = "256"
      custom_viewport_height = "224"
      custom_viewport_x = "32"
      custom_viewport_y = "8"
      (see https://github.com/RetroPie/RetroPie-Setup/wiki/Smaller-RetroArch-Screen)
    and configure /boot/config.txt to 320x240 HDMI mode to get pixel perfect rendering without blurring scaling.

    Curiously, if using quicknes emulator instead, it seems to render to a horizontally 16 pixels smaller resolution. Therefore put in
      aspect_ratio_index = "22"
      custom_viewport_width = "240"
      custom_viewport_height = "224"
      custom_viewport_x = "40"
      custom_viewport_y = "8"
    instead for pixel perfect rendering. Also in /opt/retropie/configs/all/retroarch.cfg, set

      video_fullscreen_x = "320"
      video_fullscreen_y = "240"
  */

  // The overscan values are in normalized 0.0 .. 1.0 percentages of the total width/height of the screen.
  double overscanLeft = 0.00;
  double overscanRight = 0.00;
  double overscanTop = 0.00;
  double overscanBottom = 0.00;

  // If specified, computes overscan that crops away equally much content from all sides of the source frame
  // to display the center of the source frame pixel perfect.
#ifdef DISPLAY_CROPPED_INSTEAD_OF_SCALING
  if (DISPLAY_DRAWABLE_WIDTH < width)
  {
    overscanLeft = (width - DISPLAY_DRAWABLE_WIDTH) * 0.5 / width;
    overscanRight = overscanLeft;
  }
  if (DISPLAY_DRAWABLE_HEIGHT < height)
  {
    overscanTop = (height - DISPLAY_DRAWABLE_HEIGHT) * 0.5 / height;
    overscanBottom = overscanTop;
  }
#endif

  // Overscan must be actual pixels - can't be fractional, so round the overscan %s so that they align with
  // pixel boundaries of the source image.
  overscanLeft = (double)ROUND_TO_FLOOR_INT(width * overscanLeft) / width;
  overscanRight = (double)ROUND_TO_CEIL_INT(width * overscanRight) / width;
  overscanTop = (double)ROUND_TO_FLOOR_INT(height * overscanTop) / height;
  overscanBottom = (double)ROUND_TO_CEIL_INT(height * overscanBottom) / height;

  int relevantDisplayWidth = ROUND_TO_NEAREST_INT(width * (1.0 - overscanLeft - overscanRight));
  int relevantDisplayHeight = ROUND_TO_NEAREST_INT(height * (1.0 - overscanTop - overscanBottom));
  log_printf(L_DEBUG, "Relevant source display area size with overscan cropped away: %dx%d.\n", relevantDisplayWidth, relevantDisplayHeight);

  double scalingFactorWidth = (double)DISPLAY_DRAWABLE_WIDTH/relevantDisplayWidth;
  double scalingFactorHeight = (double)DISPLAY_DRAWABLE_HEIGHT/relevantDisplayHeight;

#ifndef DISPLAY_BREAK_ASPECT_RATIO_WHEN_SCALING
  // If doing aspect ratio correct scaling, scale both width and height by equal proportions
  scalingFactorWidth = scalingFactorHeight = MIN(scalingFactorWidth, scalingFactorHeight);
#endif

  // Since display resolution must be full pixels and not fractional, round the scaling to nearest pixel size
  // (and recompute after the subpixel rounding what the actual scaling factor ends up being)
  int scaledWidth = ROUND_TO_NEAREST_INT(relevantDisplayWidth * scalingFactorWidth);
  int scaledHeight = ROUND_TO_NEAREST_INT(relevantDisplayHeight * scalingFactorHeight);
  scalingFactorWidth = (double)scaledWidth/relevantDisplayWidth;
  scalingFactorHeight = (double)scaledHeight/relevantDisplayHeight;

  displayXOffset = DISPLAY_COVERED_LEFT_SIDE + (DISPLAY_DRAWABLE_WIDTH - scaledWidth) / 2;
  displayYOffset = DISPLAY_COVERED_TOP_SIDE + (DISPLAY_DRAWABLE_HEIGHT - scaledHeight) / 2;

  excessPixelsLeft = ROUND_TO_NEAREST_INT(width * overscanLeft * scalingFactorWidth);
  excessPixelsRight = ROUND_TO_NEAREST_INT(width * overscanRight * scalingFactorWidth);
  excessPixelsTop = ROUND_TO_NEAREST_INT(height * overscanTop * scalingFactorHeight);
  excessPixelsBottom = ROUND_TO_NEAREST_INT(height * overscanBottom * scalingFactorHeight);

  gpuFrameWidth = scaledWidth;
  gpuFrameHeight = scaledHeight;
  gpuFramebufferScanlineStrideBytes = RoundUpToMultipleOf((gpuFrameWidth + excessPixelsLeft + excessPixelsRight) * 2, 32);
  gpuFramebufferSizeBytes = gpuFramebufferScanlineStrideBytes * (gpuFrameHeight + excessPixelsTop + excessPixelsBottom);

  syslog(LOG_INFO, "GPU display is %dx%d. SPI display is %dx%d with drawable area of %dx%d. Applying scaling factor horiz=%.2fx & vert=%.2fx, xOffset: %d, yOffset: %d, scaledWidth: %d, scaledHeight: %d", width, height, DISPLAY_WIDTH, DISPLAY_HEIGHT, DISPLAY_DRAWABLE_WIDTH, DISPLAY_DRAWABLE_HEIGHT, scalingFactorWidth, scalingFactorHeight, displayXOffset, displayYOffset, scaledWidth, scaledHeight);
  log_printf(L_DEBUG, "Source GPU display is %dx%d. Output SPI display is %dx%d with a drawable area of %dx%d. Applying scaling factor horiz=%.2fx & vert=%.2fx, xOffset: %d, yOffset: %d, scaledWidth: %d, scaledHeight: %d\n", width, height, DISPLAY_WIDTH, DISPLAY_HEIGHT, DISPLAY_DRAWABLE_WIDTH, DISPLAY_DRAWABLE_HEIGHT, scalingFactorWidth, scalingFactorHeight, displayXOffset, displayYOffset, scaledWidth, scaledHeight);

  log_printf(L_DEBUG, "Creating dispmanX resource of size %dx%d (aspect ratio=%f).\n", scaledWidth + excessPixelsLeft + excessPixelsRight, scaledHeight + excessPixelsTop + excessPixelsBottom, (double)(scaledWidth + excessPixelsLeft + excessPixelsRight) / (scaledHeight + excessPixelsTop + excessPixelsBottom));
  log_printf(L_INFO, "Display of %dx%d", DISPLAY_WIDTH, DISPLAY_HEIGHT);
}

void DeinitGPU()
{

}
