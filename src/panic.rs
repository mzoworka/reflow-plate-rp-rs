use core::sync::atomic::{AtomicBool, Ordering};

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    static PANICKED: AtomicBool = AtomicBool::new(false);
    cortex_m::interrupt::disable();
    if !PANICKED.load(Ordering::Relaxed) {
        PANICKED.store(true, Ordering::Relaxed);

        let core = unsafe { embassy_rp::Peripherals::steal() };
        crate::reset_peripherals_on_exception(core);

        let core = unsafe { embassy_rp::Peripherals::steal() };
        crate::print_low_level(core, info);
    }
    hard_fault();
}

pub(crate) fn hard_fault() -> ! {
    // If `UsageFault` is enabled, we disable that first, since otherwise `udf` will cause that
    // exception instead of `HardFault`.
    #[cfg(not(any(armv6m, armv8m_base)))]
    {
        const SHCSR: *mut u32 = 0xE000ED24usize as _;
        const USGFAULTENA: usize = 18;

        unsafe {
            let mut shcsr = core::ptr::read_volatile(SHCSR);
            shcsr &= !(1 << USGFAULTENA);
            core::ptr::write_volatile(SHCSR, shcsr);
        }
    }

    cortex_m::asm::udf();
}
