# TODO

## Hardware

* LEDs for larger screen
* More uses of buttons?

## Video

* Fix drawing actual data efficiently
* Improve menu
* Allow changing vars
* Building for different screens
* Border/scaling

## Audio

* Audio broken for NES emulator? (too fast?)

## Controllers

* Use udev to detect when controller disconnects?
  - Not really needed if just retrying?
* TODO sync controller if events dropped?

## Software

* Write a wrapper around Vec<u8> to provide str/CStr easily
  - more places to use PStr/PString?
* Drop priviledges and still draw to screen
    (instead of kmem)
* Improving callback mutex
    - (always accesses from same thread so no simultaneous accesses?)
    - how to do this in rust (without unsafe?) RefCell?
* Caching of symbols
* General refactoring to tidy code
* Go through expect/unwraps and remove them
* Reduce use of unsafe
* Reduce use of raw C types

## Games

* Wrap games to include commands in the menu? (e.g. quit/change dir)
* Need to test errors (and warnings) from cores better to confirm error channel works
