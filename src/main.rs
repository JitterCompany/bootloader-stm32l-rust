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

static mut CODE: extern "C" fn() = dummy;

fn run_user_program() -> ! {

    let flash_start: *const u32 = unsafe{ &__FLASH_START};
    let flash_user_start: *const u32 = unsafe{ &__FLASH_USER_START};
    //let flash_user_length: *const u32 = unsafe{ &__FLASH_USER_LENGTH};

    let flash_usercode_start_addr: u32 = flash_user_start as u32;// as u32 + 0x8000;
    //let flash_usercode_start_addr: u32 = unsafe {FLASH_START_ADDR.as_ref() as u32 + 0x8000};
    let user_stack_ptr : *const u32 = flash_usercode_start_addr as *const u32;
    let user_stack = unsafe{*user_stack_ptr};
    let user_program : *const u32 = (flash_usercode_start_addr + 4) as *const u32;

    let flash_offset : u32 = flash_user_start as u32 - flash_start as u32;


    // Create 'function pointer' to user program
    let ptr = unsafe {*user_program as *const ()};

    // Configure VTOR: use vector table from user program
    let core_periph = cortex_m::Peripherals::take().unwrap();

    unsafe {
        CODE = core::mem::transmute(ptr);

        core_periph.SCB.vtor.write(flash_offset);
        register::msp::write(user_stack);

        // Flush memory transaction
        asm::dsb();
        asm::isb();

        // Jump to user firmware
        CODE();
    }

    loop {}
}
