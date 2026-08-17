//! ARM PL011 UART driver.

const DATA_REGISTER_OFFSET: usize = 0x00;
const FLAG_REGISTER_OFFSET: usize = 0x18;
const FLAG_TRANSMIT_FIFO_FULL: u32 = 1 << 5;

/// A polling-mode ARM PL011 UART device.
pub struct Pl011 {
    base_address: usize,
}

impl Pl011 {
    /// Creates a PL011 driver for the supplied MMIO register block.
    ///
    /// # Safety
    ///
    /// `base_address` must be 32-bit aligned and point to a valid PL011
    /// MMIO register block. The caller must also guarantee exclusive access
    /// to that register block while this instance is in use.
    pub const unsafe fn new(base_address: usize) -> Self {
        Self { base_address }
    }
    /// Relinquishes this instance's exclusive access without changing device state.
    pub fn release(self) {}

    #[inline]
    fn read_register(&self, offset: usize) -> u32 {
        let address = self.base_address + offset;

        // SAFETY: The constructor contract guarantees that the base address
        // refers to a valid, aligned PL011 MMIO register block.
        unsafe { core::ptr::read_volatile(address as *const u32) }
    }

    #[inline]
    fn write_register(&mut self, offset: usize, value: u32) {
        let address = self.base_address + offset;

        // SAFETY: The constructor contract guarantees exclusive access to a
        // valid, aligned PL011 MMIO register block.
        unsafe { core::ptr::write_volatile(address as *mut u32, value) }
    }

    /// Writes one byte after waiting for space in the transmit FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while self.read_register(FLAG_REGISTER_OFFSET) & FLAG_TRANSMIT_FIFO_FULL != 0 {
            core::hint::spin_loop();
        }

        self.write_register(DATA_REGISTER_OFFSET, u32::from(byte));
    }

    /// Writes a string as its UTF-8 byte sequence.
    pub fn write_str(&mut self, text: &str) {
        for byte in text.bytes() {
            self.write_byte(byte);
        }
    }
}
