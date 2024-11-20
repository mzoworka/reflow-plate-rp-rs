#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::adc::{self, Adc, Channel, Config};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::I2C0;
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::{bind_interrupts, flash, i2c};

mod channels;
mod display;
mod heater;
mod menu;
mod panic;
mod storage;
mod temperature;
mod thermistor;
mod tools;
mod watchdog;

use display::print_low_level;
use tools::{wait_for_each_state, SyncStateChannelSender};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

/**
### Resets peripherals to safe states on exception
* For panic_handler
* Interrupts are disabled
* No panic allowed
*/
fn reset_peripherals_on_exception(peripherals: embassy_rp::Peripherals) {
    let mut mosfet = Output::new(peripherals.PIN_22, Level::Low);
    let mut led = Output::new(peripherals.PIN_25, Level::Low);

    mosfet.set_low();

    led.set_high();
    cortex_m::asm::delay(8_000_000);
    led.set_low();
    cortex_m::asm::delay(8_000_000);
    led.set_high();
    cortex_m::asm::delay(8_000_000);
    led.set_low();
}

async fn main_loop(_spawner: Spawner) -> ! {
    let peripherals = embassy_rp::init(Default::default());

    let _power_select = Output::new(peripherals.PIN_23, Level::High);

    let mut flash = flash::Flash::new_blocking(peripherals.FLASH);
    let startup_storage = storage::Storage::flash_read(&mut flash);

    let adc = Adc::new(peripherals.ADC, Irqs, Config::default());
    let adc_p26 = Channel::new_pin(peripherals.PIN_26, Pull::None);
    let thermistor = thermistor::Thermistor::new_dyze500();

    let i2c0: i2c::I2c<'_, I2C0, i2c::Async> = i2c::I2c::new_async(
        peripherals.I2C0,
        peripherals.PIN_9,
        peripherals.PIN_8,
        Irqs,
        i2c::Config::default(),
    );
    let ssd1306_i2c = ssd1306::I2CDisplayInterface::new(i2c0);
    let ssd1306_display = ssd1306::Ssd1306::new(
        ssd1306_i2c,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let mosfet = Pwm::new_output_a(
        peripherals.PWM_CH3,
        peripherals.PIN_22,
        pwm::Config::default(),
    );
    let led = Output::new(peripherals.PIN_25, Level::Low);

    let btn1 = Input::new(peripherals.PIN_2, Pull::Up);
    let btn2 = Input::new(peripherals.PIN_3, Pull::Up);
    let btn3 = Input::new(peripherals.PIN_4, Pull::Up);

    let channels = channels::Channels::new();

    let mut watchdog = watchdog::Watchdog::new(led, &channels);
    let mut storage = storage::Storage::new(&startup_storage, flash, &channels);
    let mut display = display::Display::new(ssd1306_display, &channels);
    let mut heater = heater::Heater::new(
        &startup_storage,
        adc,
        adc_p26,
        &thermistor,
        mosfet,
        &channels,
    );
    let mut menu = menu::Menu::new(&startup_storage, btn1, btn2, btn3, &channels);

    let f1 = display.display_task();
    let f2 = heater.heat_task();
    let f3 = watchdog.wd_task();
    let f4 = menu.btn_task();
    let f5 = storage.flash_task();

    let fut = join!(f1, f2, f3, f4, f5,);

    fut.await;
    panic!("not reachable");
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    main_loop(spawner).await;
}
