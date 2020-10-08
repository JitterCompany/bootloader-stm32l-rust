#![no_std]
#![no_main]

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m_rt::entry;
use cortex_m::{asm,register};
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {

    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure PA0 as output.
    let mut led = gpioa.pa0.into_push_pull_output();

    for _ in 1..3 {
        // Set the LED high one million times in a row.
        for _ in 0..1_000_00 {
            led.set_high().unwrap();
        }

        // Set the LED low one million times in a row.
        for _ in 0..1_000_00 {
            led.set_low().unwrap();
        }
    }

    run_user_program();
}

extern {
    static __FLASH_START: u32;
    static __FLASH_USER_START: u32;
    static __FLASH_USER_LENGTH: u32;
}

#[no_mangle]
extern "C" fn dummy() {}

static mut USER_PROGRAM: extern "C" fn() = dummy;

fn run_user_program() -> ! {

    // Read constants from linker script
    
    let flash_start: *const u32 = unsafe{ &__FLASH_START};
    let flash_start = flash_start as u32;

    let flash_user_start: *const u32 = unsafe{ &__FLASH_USER_START};
    let flash_user_start = flash_user_start as u32;

    // TODO: when implementing the actual bootloader, this is how to read the
    // length of user flash:
    //let flash_user_length: *const u32 = unsafe{ &__FLASH_USER_LENGTH};
    //let flash_user_length = flash_user_length as u32;

    // Get user stack address from vector table
    let user_stack : *const u32 = flash_user_start as *const u32;
    let user_stack = unsafe{*user_stack};

    // Create 'function pointer' to user program
    let user_program : *const u32 = (flash_user_start + 4) as *const u32;
    let user_program = unsafe {*user_program as *const ()};


    // Configure VTOR: use vector table from user program
    let core_periph = cortex_m::Peripherals::take().unwrap();

    unsafe {
        // Note: this must be a global as we cannot use the stack while jumping to user firmware
        USER_PROGRAM = core::mem::transmute(user_program);

        let flash_offset : u32 = flash_user_start as u32 - flash_start;
        core_periph.SCB.vtor.write(flash_offset);

        // Set stack pointer to user stack.
        // NOTE: no stack memory can be used untill the jump to user firmware.
        // (this assumes everything after this point is inlined by compiler)
        register::msp::write(user_stack);

        // Memory barrier: flush memory cache, don't reorder memory access
        asm::dsb();
        asm::isb();

        // Jump to user firmware
        USER_PROGRAM();
    }

    // user program should never return, this is never reached
    loop {}
}
