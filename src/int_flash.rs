use stm32l0xx_hal::{flash, pac, rcc};

pub struct FlashAddr {
    pub start: u32,

    pub user_start: u32,
    pub user_length: usize,
}

extern "C" {
    static __FLASH_START: u32;
    static __FLASH_USER_START: u32;
    static __FLASH_USER_LENGTH: u32;
}

pub const PAGE_SIZE: u32 = flash::PAGE_SIZE as u32;

pub fn addresses() -> FlashAddr {
    // Read constants from linker script
    let flash_start: *const u32 = unsafe { &__FLASH_START };
    let flash_start = flash_start as u32;

    let flash_user_start: *const u32 = unsafe { &__FLASH_USER_START };
    let flash_user_start = flash_user_start as u32;

    let flash_user_length: *const u32 = unsafe { &__FLASH_USER_LENGTH };
    let flash_user_length = flash_user_length as usize;

    // Struct with info about flash addresses
    FlashAddr {
        start: flash_start,
        user_start: flash_user_start,
        user_length: flash_user_length,
    }
}

pub struct InternalFlash {
    periph: flash::FLASH,
}

pub fn init(flash_peripheral: pac::FLASH, rcc: &mut rcc::Rcc) -> InternalFlash {
    InternalFlash {
        periph: flash::FLASH::new(flash_peripheral, rcc),
    }
}

impl InternalFlash {
    pub fn write_page(&mut self, page: u32, bytes: &[u8; PAGE_SIZE as usize]) {
        let addr = addresses();
        let lower_bound = addr.user_start;
        let upper_bound = addr.user_start + addr.user_length as u32;

        let addr = addr.start + page * PAGE_SIZE;
        assert!(addr >= lower_bound);
        assert!((addr + PAGE_SIZE) <= upper_bound);

        // NOTE: erase + programming is quite slow(?).
        // This could be optimized by reading the flash page first,
        // then only writing if the data is different

        // Erase page
        self.periph
            .erase_flash_page(addr as *mut u32)
            .expect("Faileld to erase flash page");

        // Verify page is all-zeroes (this is redundant, could be removed later..)
        let page_ptr = addr as *mut u8;
        for i in 0..PAGE_SIZE {
            let byte = unsafe { *page_ptr.offset(i as isize) };
            assert_eq!(byte, 0u8)
        }

        // Write two halfpages
        for h in 0..2 {
            const WORDS_PER_HALFPAGE: usize = (PAGE_SIZE / 4 / 2) as usize;
            let mut p_words: [u32; WORDS_PER_HALFPAGE] = [0; WORDS_PER_HALFPAGE];
            let byte_offset_halfpage = h * (4 * WORDS_PER_HALFPAGE);
            for w in 0..WORDS_PER_HALFPAGE {
                let byte_offset = 4 * w + byte_offset_halfpage;
                p_words[w] = bytes[byte_offset] as u32
                    | (bytes[byte_offset + 1] as u32) << 8
                    | (bytes[byte_offset + 2] as u32) << 16
                    | (bytes[byte_offset + 3] as u32) << 24;
            }

            // Program first halfpage
            //let words = [0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
            let halfpage_addr = addr + byte_offset_halfpage as u32;
            self.periph
                .write_flash_half_page(halfpage_addr as *mut u32, &p_words)
                .expect("Faileld to write a halfpage");
        }
    }
}
