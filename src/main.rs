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
    spi,
};

use spi_memory::{
    series25::Flash as ExternalFlash,
    Read,
};

mod int_flash;



#[entry]
fn main() -> ! {

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIO peripheral(s). This also enables the respective clocks (RCC)
    let gpiob = dp.GPIOB.split(&mut rcc);

    // Configure flash GPIOs
    // NOTE/TODO: on some boards, the external flash chip may be powered down
    // by default. Make sure to enable the power first in that case!!
    let mut ext_flash_cs = gpiob.pb12.into_push_pull_output();
    ext_flash_cs.set_high().unwrap();

    let spi_sclk = gpiob.pb13;
    let spi_miso = gpiob.pb14;
    let spi_mosi = gpiob.pb15;

    // Configure LED.
    let mut led = gpiob.pb5.into_push_pull_output();
    led.set_low().unwrap();


    
    // Internal Flash demo
    let page_bytes : [u8; int_flash::PAGE_SIZE as usize] = [
        0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0A,0x0B,0x0C,0x0D,0x0E,0x0F,
        0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1A,0x1B,0x1C,0x1D,0x1E,0x1F,
        0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2A,0x2B,0x2C,0x2D,0x2E,0x2F,
        0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,0x3A,0x3B,0x3C,0x3D,0x3E,0x3F,
        0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4C,0x4D,0x4E,0x4F,
        0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x59,0x5A,0x5B,0x5C,0x5D,0x5E,0x5F,
        0x60,0x61,0x62,0x63,0x64,0x65,0x66,0x67,0x68,0x69,0x6A,0x6B,0x6C,0x6D,0x6E,0x6F,
        0x70,0x71,0x72,0x73,0x74,0x75,0x76,0x77,0x78,0x79,0x7A,0x7B,0x7C,0x7D,0x7E,0x7F,
    ];

    let addr = int_flash::addresses();
    let offset = addr.user_start - addr.start;
    let page_no = offset / int_flash::PAGE_SIZE;

    let mut mcu_flash = int_flash::init(dp.FLASH, &mut rcc);
    mcu_flash.write_page(page_no, &page_bytes);


    
    // External flash readout demo

    // 4mhz appears the maximum freq that works. Probably because the main clock is at 2-4mhz?
    let spi = dp
        .SPI2
        .spi((spi_sclk, spi_miso, spi_mosi), spi::MODE_0, 4.mhz(), &mut rcc);

    let mut ext_flash = ExternalFlash::init(spi, ext_flash_cs).unwrap();
    let id = ext_flash.read_jedec_id().unwrap();
    
    // Detect SPI flash chip: must be a valid JEDEC manufacturer ID
    match id.mfr_code() {
        0x00 | 0xff => panic!("No SPI flash detected!"),
        _ => {}
    };

    // Read some data from flash memory. User firmware is responsible for writing a valid FW image here
    let mut ext_data: [u8; 128] = [0x33; 128];
    ext_flash.read(0, &mut ext_data).unwrap();
    for i in 0..ext_data.len() {
        assert!(ext_data[i] != 0x33);
    }







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
    let addr = int_flash::addresses();

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
