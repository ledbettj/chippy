# Chippy

Chippy is a chip8 emulator written in rust, which I hacked out in a few days.
It should be able to run most ROMs without issue.

![Screenshot from 2023-06-18 11-24-44](https://github.com/ledbettj/chippy/assets/805358/d7fbcdb0-d8e3-42a6-8955-815ddce9e146)


# Missing

* The delay timer and sound timer are not fully implemented.
* keypad input is currently hacky and may not work fully.
* Timing logic is hacky.
* The following instructions are unimplemented:
  * `0xF_29` set I = location of sprite for digit r[_]
  * `0xF_33` store BCD of r[_] in I[0..2]
* Super Chip-48 instructions are not supported.

## References

* http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
* https://github.com/Timendus/chip8-test-suite
* https://github.com/kripod/chip8-roms/
