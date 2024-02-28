use core::fmt::Write;
use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::Timer;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::{text, Drawable};
use simplestaticstring::{format_static, StaticString, ToStaticString};
use ssd1306::mode::DisplayConfig;

use crate::tools::SyncStateChannelReceiver;
use crate::{channels, select, temperature};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncDisplayStateEnum {
    Status(StaticString<60>),
    CurrTemp(u16),
    PeakTargetTemp(u16, temperature::TemperatureProfileEnum),
    CurrTargetTemp(u16),
    OutputEnabled(bool),
}

pub(crate) struct Display<'a> {
    channel: SyncStateChannelReceiver<'a, SyncDisplayStateEnum>,
    display: ssd1306::Ssd1306<
        ssd1306::prelude::I2CInterface<i2c::I2c<'a, I2C0, i2c::Async>>,
        ssd1306::size::DisplaySize128x64,
        ssd1306::mode::BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>,
    >,
}

impl<'a> Display<'a> {
    pub fn new(
        display: ssd1306::Ssd1306<
            ssd1306::prelude::I2CInterface<i2c::I2c<'a, I2C0, i2c::Async>>,
            ssd1306::size::DisplaySize128x64,
            ssd1306::mode::BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>,
        >,
        channels: &'a channels::Channels,
    ) -> Self {
        Self {
            channel: channels.get_display_rx(),
            display,
        }
    }

    pub async fn display_task(&mut self) -> ! {
        let rx = self.channel;
        self.display.init().expect("disp_task: init fail");

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
            let timeout = time_begin
                .checked_add(embassy_time::Duration::from_millis(100))
                .unwrap_or(time_begin);

            loop {
                let recv_fut = rx.receive();
                let sleep_fut = Timer::at(timeout);
                let select_fut = select!(recv_fut, sleep_fut,);
                let s = select_fut.await;
                match s {
                    embassy_futures::select::Either::First(stat) => match stat {
                        SyncDisplayStateEnum::Status(s) => second_line = s,
                        SyncDisplayStateEnum::CurrTemp(x) => {
                            curr_temp = format_static!("{:03}", x);
                        }
                        SyncDisplayStateEnum::PeakTargetTemp(temp, _prof) => {
                            peak_target_temp = format_static!("{:03}", temp);
                        }
                        SyncDisplayStateEnum::CurrTargetTemp(temp) => {
                            curr_target_temp = format_static!("{:03}", temp);
                        }
                        SyncDisplayStateEnum::OutputEnabled(x) => {
                            output_en = match x {
                                true => format_static!("*"),
                                false => format_static!(" "),
                            };
                        }
                    },
                    embassy_futures::select::Either::Second(_delay) => {
                        break;
                    }
                }
            }

            let first_line: StaticString<25> = format_static!(
                "{:03} -> {:03}({:03}) [{}]",
                curr_temp,
                curr_target_temp,
                peak_target_temp,
                output_en
            );

            self.display.clear_buffer();

            if embedded_graphics::text::Text::with_baseline(
                &first_line,
                Point::new(0, 0),
                text_style,
                text::Baseline::Top,
            )
            .draw(&mut self.display)
            .is_err()
            {
                //ignore: draw text failed
            }
            if embedded_graphics::text::Text::with_baseline(
                &second_line,
                Point::new(0, 16),
                text_style,
                text::Baseline::Top,
            )
            .draw(&mut self.display)
            .is_err()
            {
                //ignore: draw text failed
            }

            if self.display.flush().is_err() {
                //ignore: disp flush failed
            }
        }
    }
}

/**
### Prints exception
* For panic_handler
* Interrupts are disabled
* No panic allowed
*/
pub(crate) fn print_low_level<T>(peripherals: embassy_rp::Peripherals, info: &T)
where
    T: ToStaticString,
{
    let i2c0 = i2c::I2c::new_async(
        peripherals.I2C0,
        peripherals.PIN_9,
        peripherals.PIN_8,
        crate::Irqs,
        i2c::Config::default(),
    );
    let ssd1306_i2c = ssd1306::I2CDisplayInterface::new(i2c0);

    let mut ssd1306_display = ssd1306::Ssd1306::new(
        ssd1306_i2c,
        ssd1306::size::DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let text_style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();

    if ssd1306_display.init().is_ok() {
        ssd1306_display.clear_buffer();

        if embedded_graphics::text::Text::with_baseline(
            "Fatal",
            Point::new(0, 0),
            text_style,
            text::Baseline::Top,
        )
        .draw(&mut ssd1306_display)
        .is_err()
        {
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

        if embedded_graphics::text::Text::with_baseline(
            &info_str_wrapped,
            Point::new(0, 16),
            text_style,
            text::Baseline::Top,
        )
        .draw(&mut ssd1306_display)
        .is_err()
        {
            //ignore: draw text failed
        }

        if ssd1306_display.flush().is_err() {
            //ignore: disp flush failed
        }
    }
}
