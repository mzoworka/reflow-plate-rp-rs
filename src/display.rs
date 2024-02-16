use core::fmt::Write;
use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::Timer;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::{self, text, Drawable};
use simplestaticstring::{format_static, StaticString, ToStaticString};
use ssd1306;
use ssd1306::mode::DisplayConfig;
use static_cell::StaticCell;

use crate::{temperature, SyncStateChannelReceiver, select};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncDisplayStateEnum {
    Status(StaticString<60>),
    CurrTemp(u16),
    PeakTargetTemp(u16, temperature::TemperatureProfileEnum),
    CurrTargetTemp(u16),
    OutputEnabled(bool),
}

pub(crate) async fn display_task(display: &mut ssd1306::Ssd1306<ssd1306::prelude::I2CInterface<i2c::I2c<'static, I2C0, i2c::Async>>, ssd1306::size::DisplaySize128x64, ssd1306::mode::BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>>, rx: SyncStateChannelReceiver<'_, SyncDisplayStateEnum>) -> ! {
    display.init().expect("disp_task: init fail");

    let text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();
    
    let mut second_line: StaticString<60> = format_static!("Welcome!");

    let mut curr_temp: StaticString<5> = format_static!("???");
    let mut curr_target_temp: StaticString<3> = format_static!("000");
    let mut peak_target_temp: StaticString<3> = format_static!("000");
    let mut output_en: StaticString<1> = format_static!(" ");
   
    loop {
        let time_begin = embassy_time::Instant::now();
        let timeout = time_begin.checked_add(embassy_time::Duration::from_millis(100)).unwrap_or(time_begin);

        loop
        {
            let recv_fut = rx.receive();
            let sleep_fut = Timer::at(timeout);
            let select_fut = select!(recv_fut, sleep_fut, );
            let s = select_fut.await;
            match s {
                embassy_futures::select::Either::First(stat) => match stat {
                    SyncDisplayStateEnum::Status(s) => {second_line = s},
                    SyncDisplayStateEnum::CurrTemp(x) => {
                        curr_temp = format_static!("{:03}", x);
                    },
                    SyncDisplayStateEnum::PeakTargetTemp(temp, _prof) => {
                        peak_target_temp = format_static!("{:03}", temp);
                    },
                    SyncDisplayStateEnum::CurrTargetTemp(temp) => {
                        curr_target_temp = format_static!("{:03}", temp);
                    },
                    SyncDisplayStateEnum::OutputEnabled(x) => {
                        output_en = match x {
                            true => format_static!("*"),
                            false => format_static!(" "),
                        };               
                    },
                },
                embassy_futures::select::Either::Second(_delay) => { break; },
            }
        }

        let first_line: StaticString<25> = format_static!("{:03} -> {:03}({:03}) [{}]", curr_temp, curr_target_temp, peak_target_temp, output_en);        

        display.clear_buffer();

        if embedded_graphics::text::Text::with_baseline(&first_line, Point::new(0, 0), text_style, text::Baseline::Top).draw(display).is_err() {
            //ignore: draw text failed
        }
        if embedded_graphics::text::Text::with_baseline(&second_line, Point::new(0, 16), text_style, text::Baseline::Top).draw(display).is_err() {
            //ignore: draw text failed
        }

        if display.flush().is_err() {
            //ignore: disp flush failed
        }
    }
}


/**
 ### Prints exception
 * For panic_handler
 * Interrupts are disabled
 * No panic allowed
 */
pub(crate) fn print_low_level<'a, T>(peripherals: embassy_rp::Peripherals, info: &'a T)
where T: ToStaticString
{
    let i2c0 = i2c::I2c::new_async(peripherals.I2C0, peripherals.PIN_9, peripherals.PIN_8, crate::Irqs, i2c::Config::default());
    let ssd1306_i2c = ssd1306::I2CDisplayInterface::new(i2c0);
    
    static SSD1306_DISP: StaticCell<ssd1306::Ssd1306<ssd1306::prelude::I2CInterface<i2c::I2c<'static, I2C0, i2c::Async>>, ssd1306::prelude::DisplaySize128x64, ssd1306::mode::BufferedGraphicsMode<ssd1306::prelude::DisplaySize128x64>>> = StaticCell::new();
    let ssd1306_display = SSD1306_DISP.init(ssd1306::Ssd1306::new(ssd1306_i2c, ssd1306::size::DisplaySize128x64, ssd1306::rotation::DisplayRotation::Rotate0).into_buffered_graphics_mode());

    let text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();
    
    if ssd1306_display.init().is_ok() {
        ssd1306_display.clear_buffer();

        if embedded_graphics::text::Text::with_baseline("Fatal" , Point::new(0, 0), text_style, text::Baseline::Top).draw(ssd1306_display).is_err() {
            //ignore: draw text failed
        }

        let info_str = info.to_static_string::<128>().unwrap_or_default();
        let info_str_chunks = info_str.as_slice().chunks(20);
        let mut info_str_wrapped = StaticString::<128>::default();
        for chunk in info_str_chunks {
            if info_str_wrapped.try_extend_from_slice(chunk).is_err() {
                break;
            }
            if info_str_wrapped.try_extend_from_slice(b"\n").is_err() {
                break;
            }
        }

        if embedded_graphics::text::Text::with_baseline(&info_str_wrapped, Point::new(0, 16), text_style, text::Baseline::Top).draw(ssd1306_display).is_err() {
            //ignore: draw text failed
        }

        if ssd1306_display.flush().is_err() {
            //ignore: disp flush failed
        }
    }
    
}
