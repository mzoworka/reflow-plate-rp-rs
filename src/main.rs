#![no_std]
#![no_main]

mod panic;
mod menu;
mod display;
mod storage;
mod tools;
mod heater;
mod watchdog;
mod thermistor;
mod temperature;

use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler};
use embassy_rp::peripherals::I2C0;
use embassy_rp::pwm::{Pwm, self};
use embassy_rp::{bind_interrupts, i2c, flash};
use embassy_rp::gpio::{Pull, Output, Level, Input,};
use display::{print_low_level, SyncDisplayStateEnum};
use tools::{SyncStateChannelReceiver, SyncStateChannelSender, wait_for_each_state};
use crate::heater::SyncHeatStateEnum;
use crate::storage::SyncStorageStateEnum;
use crate::tools::SyncStateChannel;
use crate::watchdog::SyncWdStateEnum;

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    ADC_IRQ_FIFO => InterruptHandler;
});

/**
 ### Resets peripherals to safe states on exception
 * For panic_handler
 * Interrupts are disabled
 * No panic allowed
 */
fn reset_peripherals_on_exception(peripherals: embassy_rp::Peripherals)
{    
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

    let display_channel = SyncStateChannel::<SyncDisplayStateEnum>::new();
    let wd_channel = SyncStateChannel::<SyncWdStateEnum>::new();
    let storage_channel = SyncStateChannel::<SyncStorageStateEnum>::new();
    let heat_channel = SyncStateChannel::<SyncHeatStateEnum>::new();

    let mut flash = flash::Flash::new_blocking(peripherals.FLASH);
    let startup_storage = storage::flash_read(&mut flash);

    let mut adc = Adc::new(peripherals.ADC, Irqs, Config::default());
    let mut p26 = Channel::new_pin(peripherals.PIN_26, Pull::None);
    let thermistor = thermistor::Thermistor::new_dyze500();

    let i2c0: i2c::I2c<'_, I2C0, i2c::Async> = i2c::I2c::new_async(peripherals.I2C0, peripherals.PIN_9, peripherals.PIN_8, Irqs, i2c::Config::default());
    let ssd1306_i2c = ssd1306::I2CDisplayInterface::new(i2c0);
    let mut ssd1306_display = ssd1306::Ssd1306::new(ssd1306_i2c, ssd1306::size::DisplaySize128x64, ssd1306::rotation::DisplayRotation::Rotate0).into_buffered_graphics_mode();

    let mut mosfet = Pwm::new_output_a(peripherals.PWM_CH3, peripherals.PIN_22, pwm::Config::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);

    let mut btn1 = Input::new(peripherals.PIN_2, Pull::Up);
    let mut btn2 = Input::new(peripherals.PIN_3, Pull::Up);
    let mut btn3 = Input::new(peripherals.PIN_4, Pull::Up);

    let f1 = display::display_task(&mut ssd1306_display, display_channel.receiver());
    let f2 = heater::heat_task(&startup_storage, &mut adc, &mut p26, &thermistor, &mut mosfet, display_channel.sender(), wd_channel.sender(), heat_channel.receiver());
    let f3 = watchdog::wd_task(wd_channel.receiver(), &mut led);
    let f4 = menu::btn_task(&startup_storage, &mut btn1, &mut btn2, &mut btn3, display_channel.sender(), heat_channel.sender(), storage_channel.sender());
    let f5 = storage::flash_task(&startup_storage, &mut flash, storage_channel.receiver());
    
    let fut = join!(
        f1,
        f2,
        f3,
        f4,
        f5,
    );

    fut.await;
    panic!("not reachable");
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    main_loop(spawner).await;
}
