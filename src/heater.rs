use embassy_time::Timer;
use embassy_rp::pwm::{Pwm, self};
use embassy_rp::adc::{Adc, Channel};
use pid_lite::Controller;

use crate::display::SyncDisplayStateEnum;
use crate::thermistor::Thermistor;
use crate::watchdog::SyncWdStateEnum;
use crate::{SyncStateChannelReceiver, SyncStateChannelSender, select, storage};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncHeatStateEnum {
    TargetTemp(u16, u8),
    Pid((bool, f32, f32, f32)),
}

pub(crate) async fn heat_task(startup_storage: &storage::Storage, adc: &'_ mut Adc<'_, embassy_rp::adc::Async>, temp_ch: &'_ mut Channel<'_>, thermistor: &Thermistor, mosfet: &'_ mut Pwm<'_, embassy_rp::peripherals::PWM_CH3>, display_tx: SyncStateChannelSender<'_, SyncDisplayStateEnum>, wd_tx: SyncStateChannelSender<'_, SyncWdStateEnum>, heat_rx: SyncStateChannelReceiver<'_, SyncHeatStateEnum>) -> ! {
    let mut target_temp = 0;
    let mut pid_use = startup_storage.pid;
    let mut pid_p = startup_storage.pid_p;
    let mut pid_i = startup_storage.pid_i;
    let mut pid_d = startup_storage.pid_d;
    let mut controller = Controller::new(target_temp as f64, pid_p as f64, pid_i as f64, pid_d as f64);
    let mut time_begin = embassy_time::Instant::now();
    let mut pwm_config = pwm::Config::default();
    loop {
        //recv updates or sleep
        let recv_fut = heat_rx.receive();
        let sleep_fut = Timer::after_millis(100);
        let select_fut = select!(recv_fut, sleep_fut, );
        match select_fut.await {
            embassy_futures::select::Either::First(state) => match state {
                SyncHeatStateEnum::TargetTemp(temp, prof) => {
                    target_temp = temp;
                    controller.set_target(temp as f64);
                }
                SyncHeatStateEnum::Pid(x) => {
                    pid_use = x.0;
                    pid_p = x.1;
                    pid_i = x.2;
                    pid_d = x.3;
                    controller.set_proportional_gain(pid_p as f64);
                    controller.set_integral_gain(pid_i as f64);
                    controller.set_derivative_gain(pid_d as f64);
                    controller.reset();
                },
            },
            embassy_futures::select::Either::Second(()) => {},
        }

        //read current temp
        let temp_val = adc.read(temp_ch).await.expect("heat_task: temp fail");
        let current_temp = thermistor.calc_temp(temp_val) as u16;

        let time_elapsed = embassy_time::Instant::now() - time_begin;
        if time_elapsed.as_millis() > 10 {
            //calc corrections
            pwm_config.compare_a = if !pid_use {
                if current_temp < target_temp {
                    pwm_config.top
                } else {
                    0
                }
            } else {
                controller.update_elapsed(current_temp as f64, time_elapsed.into()) as u16
            };

            //set mosfet
            mosfet.set_config(&pwm_config);

            time_begin = embassy_time::Instant::now();
            
            //send updates
            if display_tx.try_send(SyncDisplayStateEnum::CurrTemp(current_temp)).is_err() {
                //ignore: msg dropped
            }
            if display_tx.try_send(SyncDisplayStateEnum::OutputEnabled(pwm_config.compare_a > 0)).is_err() {
                //ignore: msg dropped
            }
        }
        
        //feed wd
        wd_tx.try_send(SyncWdStateEnum::HeatTask).expect("heat_task: wdtx fail");
        
    }
}
