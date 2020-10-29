#![no_std]
#![no_main]

use panic_halt as _;
use core::cmp;

use p256::{
    ecdsa::{
        VerifyKey,
        Signature,
    },
    elliptic_curve::FieldBytes,
    elliptic_curve::scalar,
};

use sha2::{Sha256, Digest};
use signature::DigestVerifier;



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
    delay::Delay,
};

use spi_memory::{
    series25::Flash as ExternalFlash,
    Read,
};

mod int_flash;
mod pubkey;


fn parse_meta(buffer: [u8; 8]) -> FirmwareMeta
{
    FirmwareMeta {
        image_type: (buffer[1] as u16) << 8 | (buffer[0] as u16),
        extra_file_count: (buffer[3] as u16) << 8 | (buffer[2] as u16),
        fw_len: ((buffer[7] as u32) << 24
            | (buffer[6] as u32) << 16
            | (buffer[5] as u32) << 8
            | (buffer[4] as u32)) as usize,
    }
}

const FW_META_OFFSET: u32 = 0xC0;
const FW_SIGNATURE_LEN: usize = 64;

struct FirmwareMeta {
    image_type: u16,
    extra_file_count: u16,
    fw_len: usize,
}


#[entry]
fn main() -> ! {

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut delay = cp.SYST.delay(rcc.clocks);

    // Acquire the GPIO peripheral(s). This also enables the respective clocks (RCC)
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    // Board-dependent GPIO mapping. TODO surely this can be done in a nicer way...
    // NOTE: build with '--features "board-XXX"' to select one of the supported boards
    #[cfg(feature = "board-6001-devkit")]
    let mut led = gpiob.pb5.into_push_pull_output();
    #[cfg(feature = "board-6001-devkit")]
    let mut ext_flash_cs = gpiob.pb12.into_push_pull_output();

    #[cfg(feature = "board-6001-sensor")]
    let mut led = gpioa.pa0.into_push_pull_output();
    #[cfg(feature = "board-6001-sensor")]
    let mut ext_flash_cs = gpiob.pb5.into_push_pull_output();

    #[cfg(feature = "board-6001-gateway")]
    let mut led = gpioa.pa0.into_push_pull_output();
    #[cfg(feature = "board-6001-gateway")]
    let mut ext_flash_cs = gpioa.pa11.into_push_pull_output();  
    
    
    // SPI flash GPIO
    ext_flash_cs.set_high().unwrap();
    let spi_sclk = gpiob.pb13;
    let spi_miso = gpiob.pb14;
    let spi_mosi = gpiob.pb15;

    // LED GPIO
    led.set_low().unwrap();


    // 4MHz appears to be the maximum freq that works.
    // Probably because the main clock is at 2-4MHz??
    let spi = dp
        .SPI2
        .spi((spi_sclk, spi_miso, spi_mosi), spi::MODE_0, 1.mhz(), &mut rcc);

    let mut ext_flash = ExternalFlash::init(spi, ext_flash_cs).unwrap();
    let id = ext_flash.read_jedec_id().unwrap();
    
    // Detect SPI flash chip: must be a valid JEDEC manufacturer ID
    match id.mfr_code() {
        0x00 | 0xff => panic!("No SPI flash detected!"),
        0x1F => (),
        _ => panic!("Unknown SPI flash detected!"),
    };

    let mut ok = true;


    // Read metadata from external flash
    let mut buffer: [u8; 8] = [0; 8];
    ext_flash.read(FW_META_OFFSET, &mut buffer).unwrap();
    let meta: FirmwareMeta = parse_meta(buffer);
    if meta.image_type != 0x3801 || meta.extra_file_count != 0 {
        ok = false;
    }

    // Candidate image: firmware size must be within bounds
    if meta.fw_len < FW_SIGNATURE_LEN 
        || meta.fw_len < FW_META_OFFSET as usize
        || (meta.fw_len + FW_SIGNATURE_LEN) > int_flash::addresses().user_length {
            ok = false;
    }

    // Candidate image: check signature 
    if ok {
        blink_start_update(&mut delay, &mut led);

        const BLOCK_SIZE: usize = 128;
        let fw_len: usize = meta.fw_len - FW_SIGNATURE_LEN;

        let mut hasher = Sha256::new();
        let mut bytes_remaining:usize = fw_len;
        let mut offset:usize = 0;
        while bytes_remaining > 0 {
            let mut buffer: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
    
            let len:usize = cmp::min(bytes_remaining, BLOCK_SIZE);
    
            ext_flash.read(offset as u32, &mut buffer[0..len]).unwrap();
            bytes_remaining-= len;
            offset+= len;
            
            hasher.update(&buffer[0..len]);
        }

        // Read & verify ECC P-256 signature
        let mut sig_bytes : [u8; 64] = [1;64];
        ext_flash.read(fw_len as u32, &mut sig_bytes).unwrap();
        let r = *FieldBytes::<p256::NistP256>::from_slice(&sig_bytes[0..32]);
        let r = scalar::NonZeroScalar::<p256::NistP256>::from_repr(r).unwrap();
        let s = FieldBytes::<p256::NistP256>::from_slice(&sig_bytes[32..64]);
        let s = scalar::NonZeroScalar::<p256::NistP256>::from_repr(*s).unwrap();
        let sig = Signature::from_scalars(r, s).unwrap();

        let pubkey = pubkey::FW_SIGN_PUBKEY;
        let verify_key = VerifyKey::new(&pubkey).unwrap();
        if !verify_key.verify_digest(hasher, &sig).is_ok() {
            ok = false;
        }
    }

    // Copy image to internal flash (TODO only do this if ext_flash != int_flash)
    if ok {
        let mut mcu_flash = int_flash::init(dp.FLASH, &mut rcc);
        let addr = int_flash::addresses();
        let flash_user_offset = addr.user_start - addr.start;

        // Total length of firmware image in bytes (incl signature)
        let mut bytes_remaining:usize = meta.fw_len;
        let mut ext_offset:usize = 0;

        while bytes_remaining > 0 {  
            let len:usize = cmp::min(bytes_remaining, int_flash::PAGE_SIZE as usize);

            // Read up to one page of data from ext_flash
            let mut buffer: [u8; int_flash::PAGE_SIZE as usize] = [0; int_flash::PAGE_SIZE as usize];    
            ext_flash.read(ext_offset as u32, &mut buffer[0..len]).unwrap();

            // Copy data to internal mcu_flash
            // NOTE: we always write the whole page, the last page is effectively zero-padded
            // (because buffer is initialized to zeroes)
            let int_page_no: u32 = (ext_offset as u32 + flash_user_offset) / int_flash::PAGE_SIZE;
            mcu_flash.write_page(int_page_no, &buffer);

            bytes_remaining-= len;
            ext_offset+= len;      
        }
    }

    if ok {
        blink_ok(&mut delay, &mut led);
    } else {
        blink_error(&mut delay, &mut led);
    }

    
    


    
    run_user_program(cp.SCB);
}

fn blink_start_update<LED: OutputPin>(delay: &mut Delay, led: &mut LED)
{
    led.set_high().ok();
    delay.delay_ms(300_u32);
    led.set_low().ok();
}
fn blink_ok<LED: OutputPin>(delay: &mut Delay, led: &mut LED)
{
    for _ in 0..2 {
        led.set_high().ok();
        delay.delay_ms(300_u32);

        led.set_low().ok();
        delay.delay_ms(600_u32);
    }
}
fn blink_error<LED: OutputPin>(delay: &mut Delay, led: &mut LED)
{
    for _ in 0..3 {
        led.set_high().ok();
        delay.delay_ms(50_u32);

        led.set_low().ok();
        delay.delay_ms(40_u32);
    }
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
