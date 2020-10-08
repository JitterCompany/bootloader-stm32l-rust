# bootloader-stm32l-rust

Simple bootloader for STM32L0 microcontrollers, written in Rust


## Building

**NOTE** this code **MUST** be built as a release. Compiler optimizations
are required to make the jump to usercode work correctly.
To make the bootloader work, some very low stuff needs to be done, including
writing directly to the `MSP` register.
The `register::msp::write()` call only works correctly if it is correctly inlined
by the compiler, otherwise the function call overhead itself messes up the stack pointer.

If you know a better way to do this (using stable rust), please let us know!

## Design philosophy

The goal for this bootloader is to be simple and reliable. The bootloader should
only do tasks that are not suitable to implement in user firmware.

Even for advanced scenario's such as Over-The-Air updates, the bootloader can
still be kept simple. The update process can be split in two steps:

1. The user firmware somehow receives a new update candidate (this could be OTA,
via UART, USB, or any other application-specific way).

2. The user firmware triggers a reset-to-bootloader. Bootloader verifies if the
    new firmware image is valid and then performs the update.

This keeps the bootloader compact and simple, while allowing a lot of flexibility
for the user firmware on how and when updates should be performed.
