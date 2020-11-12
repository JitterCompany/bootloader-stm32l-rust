# bootloader-stm32l-rust

Simple bootloader for STM32L0 microcontrollers, written in Rust


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


## Code signing

This bootloader expects the firmware to be signed with `ECC P-256`, sometimes refered to as `prime256v1`.
The signature is appended to the end of the file as a raw 64-byte signature.

### Key pair
You need to create a private-public key pair. The private key is used to sign firmware images
and should be kept secret (e.g. keep it offline, ideally on a hardware smartcard/yubikey).
[See docs on how to do it with a Yubikey](https://github.com/JitterCompany/bootloader-stm32l-rust/blob/master/docs/setup-yubikey.md)

The public key (in .pem format) should be stored as `pubkey.pem` (see `pubkey.pem.example` in the repository).


## Building

### Compiler optimizations

**NOTE** this code **MUST** be built as a release. Compiler optimizations
are required to make the jump to usercode work correctly.
To make the bootloader work, some very low-level stuff needs to be done, including
writing directly to the `MSP` register.
The `register::msp::write()` call only works correctly if it is correctly inlined
by the compiler, otherwise the function call overhead itself messes up the stack pointer.

If you know a better way to do this (using stable rust), please let us know!
Also note that this project is built with optimization flag "s". This is required in order
to make the firmware fit in 32K of flash.

### Board selection

The bootloader supports multiple target boards. Board-specific source code is
specified by `#[cfg(feature = "board-XXX")]`. Pass the desired board to cargo/bobbin
to build the right version. For example:

```
cargo build --release --features "board-6001-devkit"
```
Builds the 'board-6001-devkit' specific version.

You can also use bobbin to immediately flash the binary as well
```
bobbin load --bin mcu-bootloader-rust --release --features "board-6001-devkit"
```


