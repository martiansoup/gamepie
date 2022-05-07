# GAMEpie

GAMEpie is a retro game emulation system for Raspberry Pi, using a mini LCD screen and speaker. Emulation is provided by libretro cores.

## Dependencies

Nintendo controller support via the [dkms-hid-nintendo](https://github.com/nicman23/dkms-hid-nintendo) kernel module. (Requires the `raspberrypi-kernel-headers` package).

Needs the `cmake libevdev-dev libclang-dev libsdl2-dev` packages to compile Rust dependencies.

## Bill of Materials

* Raspberry Pi Zero 2 W
* Screen + Audio: [Pirate Audio Speaker](https://shop.pimoroni.com/products/pirate-audio-mini-speaker?variant=31189753692243)
  - plus header extension to fix case
* Battery charger: [Adafruit PowerBoost 500](https://www.adafruit.com/product/1944)
* Battery: [500mAH LiPo](https://shop.pimoroni.com/products/lipo-battery-pack?variant=20429082055)
* [Power switch](https://shop.pimoroni.com/products/lilypad-e-sewing-protosnap?variant=1563028488202)
* Controller: [8bitdo Zero 2](https://www.8bitdo.com/zero2/)
  - 64-bit OS seemed to have some issues with staying paired and required [moving link keys](https://unix.stackexchange.com/questions/255509/bluetooth-pairing-on-dual-boot-of-windows-linux-mint-ubuntu-stop-having-to-p)

## Known Issues

* NES emulation sound appears to be too fast.

## Boot config

Uses the following boot config:

```
dtparam=audio=off
gpio=13=op,dl
gpio=25=op,dl
gpio=26=ip,np # Low battery input
dtoverlay=hifiberry-dac
```

## Credits

SPI Screen driving code adapted from [fbcp-ili9341](https://github.com/juj/fbcp-ili9341) by
Jukka Jylänki, under the MIT license.

Icons are [225 Icons](https://vectorpixelstar.itch.io/225-icons) by VectorPixelStar. Under the [CC By-ND 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.en) license.

[libretro API](https://github.com/libretro/RetroArch/blob/master/libretro-common/include/libretro.h) by the RetroArch team.

OpenSCAD [Pi Zero case](https://www.thingiverse.com/thing:4836001) by Naj. Under the [CC By 4.0](https://creativecommons.org/licenses/by/4.0/) license.
