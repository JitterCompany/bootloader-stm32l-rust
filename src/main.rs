#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use cortex_m::{
    asm,
    register,
    peripheral::SCB,
};
use stm32l0xx_hal::{
    pac,
    prelude::*,
    rcc::Config,
    flash::{FLASH, PAGE_SIZE}
};

// flash stuff

struct FlashAddr {
    start : u32,

    user_start : u32,
    user_length : usize,
}

extern {
    static __FLASH_START: u32;
    static __FLASH_USER_START: u32;
    static __FLASH_USER_LENGTH: u32;
}
fn flash_addr() -> FlashAddr {

    // Read constants from linker script
    let flash_start: *const u32 = unsafe{ &__FLASH_START};
    let flash_start = flash_start as u32;
    
    let flash_user_start: *const u32 = unsafe{ &__FLASH_USER_START};
    let flash_user_start = flash_user_start as u32;
    
    let flash_user_length: *const u32 = unsafe{ &__FLASH_USER_LENGTH};
    let flash_user_length = flash_user_length as usize;

    // Struct with info about flash addresses
    FlashAddr {
        start: flash_start,
        user_start: flash_user_start,
        user_length: flash_user_length,
    }
}


// end flash stuff

#[entry]
fn main() -> ! {

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut flash = FLASH::new(dp.FLASH, &mut rcc);

    let addr = flash_addr();

    // Erase page
    flash
        .erase_flash_page(addr.user_start as *mut u32)
        .expect("Faileld to erase flash page");
    
    // Verify page is all-zeroes
    let flash_user_start_ptr = addr.user_start as *mut u8;
    for i in 0..PAGE_SIZE {
        let byte = unsafe{*flash_user_start_ptr.offset(i as isize)};
        assert_eq!(byte, 0u8)
    }

    // Program first halfpage
    let words = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
    flash
        .write_flash_half_page(addr.user_start as *mut u32, &words)
        .expect("Failed to write flash (half-)page");
    







    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpiob= dp.GPIOB.split(&mut rcc);

    // Configure PA0 as output.
    let mut led = gpiob.pb5.into_push_pull_output();

    let mut delay = cp.SYST.delay(rcc.clocks);


    for _ in 1..1000000000 {
        led.set_high().unwrap();
        delay.delay_ms(70u32);

        led.set_low().unwrap();
        delay.delay_ms(300u16);
    }
    
    run_user_program(cp.SCB);
}


#[no_mangle]
extern "C" fn dummy() {}

static mut USER_PROGRAM: extern "C" fn() = dummy;

fn run_user_program(scb: SCB) -> ! {

    // Get important flash addresses
    let addr = flash_addr();

    // Get user stack address from vector table
    let user_stack : *const u32 = addr.user_start as *const u32;
    let user_stack = unsafe{*user_stack};

    // Create 'function pointer' to user program
    let user_program : *const u32 = (addr.user_start + 4) as *const u32;
    let user_program = unsafe {*user_program as *const ()};

    unsafe {
        // Note: this must be a global as we cannot use the stack while jumping to user firmware
        USER_PROGRAM = core::mem::transmute(user_program);

        let vector_table_offset : u32 = addr.user_start - addr.start;
        
        // Relocate vector table: use vector table from user program
        scb.vtor.write(vector_table_offset);

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
