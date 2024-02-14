use embassy_time::Timer;
use embassy_rp::gpio::Output;

use crate::{SyncStateChannelReceiver, select, wait_for_each_state};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncWdStateEnum {
    HeatTask,
}

pub(crate) async fn wd_task(rx: SyncStateChannelReceiver<'_, SyncWdStateEnum>, led: &'_ mut Output<'_, embassy_rp::peripherals::PIN_25>) -> ! {
    loop {
        let recv_fut = wait_for_each_state([SyncWdStateEnum::HeatTask], rx);
        let sleep_fut = Timer::after_millis(1000);
        let select_fut = select!(recv_fut, sleep_fut, );
        match select_fut.await {
            embassy_futures::select::Either::First(_x) => {},
            embassy_futures::select::Either::Second(()) => panic!("wd failed"),
        }

        led.toggle();

    }
}
