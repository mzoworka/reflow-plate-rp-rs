use embassy_rp::gpio::Output;
use embassy_time::Timer;

use crate::{channels, select, tools::SyncStateChannelReceiver, wait_for_each_state};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncWdStateEnum {
    HeatTask,
}

pub(crate) struct Watchdog<'a> {
    channel: SyncStateChannelReceiver<'a, SyncWdStateEnum>,
    led: Output<'a>,
}

impl<'a> Watchdog<'a> {
    pub fn new(led: Output<'a>, channels: &'a channels::Channels) -> Self {
        Self {
            channel: channels.get_watchdog_rx(),
            led,
        }
    }

    pub async fn wd_task(&mut self) -> ! {
        let rx = self.channel;
        loop {
            let recv_fut = wait_for_each_state([SyncWdStateEnum::HeatTask], rx);
            let sleep_fut = Timer::after_millis(1000);
            let select_fut = select!(recv_fut, sleep_fut,);
            match select_fut.await {
                embassy_futures::select::Either::First(_x) => {}
                embassy_futures::select::Either::Second(()) => panic!("wd failed"),
            }

            self.led.toggle();
        }
    }
}
