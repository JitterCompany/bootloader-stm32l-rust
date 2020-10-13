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
    let flash = FLASH::new(dp.FLASH, &mut rcc);

    
    let page_bytes : [u8; PAGE_SIZE as usize] = [
        0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0A,0x0B,0x0C,0x0D,0x0E,0x0F,
        0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1A,0x1B,0x1C,0x1D,0x1E,0x1F,
        0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2A,0x2B,0x2C,0x2D,0x2E,0x2F,
        0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,0x3A,0x3B,0x3C,0x3D,0x3E,0x3F,
        0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4C,0x4D,0x4E,0x4F,
        0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x59,0x5A,0x5B,0x5C,0x5D,0x5E,0x5F,
        0x60,0x61,0x62,0x63,0x64,0x65,0x66,0x67,0x68,0x69,0x6A,0x6B,0x6C,0x6D,0x6E,0x6F,
        0x70,0x71,0x72,0x73,0x74,0x75,0x76,0x77,0x78,0x79,0x7A,0x7B,0x7C,0x7D,0x7E,0x7F,
    ];

    let addr = flash_addr();
    let offset = addr.user_start - addr.start;
    let page_no = offset / PAGE_SIZE;
    flash_write_page(flash, page_no, &page_bytes);







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


fn flash_write_page(mut flash: stm32l0xx_hal::flash::FLASH,
    page: u32, bytes: &[u8; PAGE_SIZE as usize]) {

    let addr = flash_addr();
    let lower_bound = addr.user_start;
    let upper_bound = addr.user_start + addr.user_length as u32;

    let addr = addr.start + page * PAGE_SIZE;
    assert!(addr >= lower_bound);
    assert!((addr+PAGE_SIZE) <= upper_bound);

    // NOTE: erase + programming is quite slow(?).
    // This could be optimized by reading the flash page first,
    // then only writing if the data is different

    // Erase page
    flash
        .erase_flash_page(addr as *mut u32)
        .expect("Faileld to erase flash page");
    
    // Verify page is all-zeroes (this is redundant, could be removed later..)
    let page_ptr = addr as *mut u8;
    for i in 0..PAGE_SIZE {
        let byte = unsafe{*page_ptr.offset(i as isize)};
        assert_eq!(byte, 0u8)
    }


    // Write two halfpages
    for h in 0..2 {
        const WORDS_PER_HALFPAGE: usize = (PAGE_SIZE/4/2) as usize;
        let mut p_words : [u32; WORDS_PER_HALFPAGE] = [0; WORDS_PER_HALFPAGE];
        let byte_offset_halfpage = h*(4*WORDS_PER_HALFPAGE);
        for w in 0..WORDS_PER_HALFPAGE {
            let byte_offset = 4*w + byte_offset_halfpage;
            p_words[w] = bytes[byte_offset] as u32
                | (bytes[byte_offset+1] as u32) << 8
                | (bytes[byte_offset+2] as u32) << 16
                | (bytes[byte_offset+3] as u32) << 24;
        }

        // Program first halfpage
        //let words = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
        let halfpage_addr = addr + byte_offset_halfpage as u32;
        flash
            .write_flash_half_page(halfpage_addr  as *mut u32, &p_words)
            .expect("Faileld to write a halfpage");

    }
    

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
