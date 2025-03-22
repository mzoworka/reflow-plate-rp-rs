use core::{
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
};

struct PanicInfoWrap<'a>(&'a core::panic::PanicInfo<'a>);

impl Display for PanicInfoWrap<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("panic: ")?;
        self.0.message().fmt(f)?;
        f.write_str("@\n")?;
        self.0.location().map(|loc| {
            let filename = loc.file();
            let len = if filename.len() > 50 {
                50
            } else {
                filename.len()
            };
            f.write_str(&filename[filename.len() - len..])?;
            f.write_str(":")?;
            loc.line().fmt(f)
        });
        Ok(())
    }
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    static PANICKED: AtomicBool = AtomicBool::new(false);
    cortex_m::interrupt::disable();
    if !PANICKED.load(Ordering::Relaxed) {
        PANICKED.store(true, Ordering::Relaxed);

        let core = unsafe { embassy_rp::Peripherals::steal() };
        crate::reset_peripherals_on_exception(core);

        let core = unsafe { embassy_rp::Peripherals::steal() };
        crate::print_low_level(core, &PanicInfoWrap(info));
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
