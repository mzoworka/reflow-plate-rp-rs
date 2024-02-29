use embassy_rp::adc::{Adc, Channel};
use embassy_rp::pwm::{self, Pwm};
use embassy_time::Timer;
use pid_lite::Controller;

use crate::display::SyncDisplayStateEnum;
use crate::thermistor::Thermistor;
use crate::tools::SyncStateChannelReceiver;
use crate::watchdog::SyncWdStateEnum;
use crate::{channels, select, storage, temperature, SyncStateChannelSender};

#[derive(Debug, PartialEq)]
pub(crate) enum SyncHeatStateEnum {
    TargetTemp(u16, temperature::TemperatureProfileEnum),
    Pid {
        pid: bool,
        pid_p: f32,
        pid_i: f32,
        pid_d: f32,
    },
    TempSettings {
        wait_time: f32,
        extra_time: f32,
        temp_lead_offset: i16,
        temp_offset: i16,
    },
}

pub(crate) struct Heater<'a> {
    channel: SyncStateChannelReceiver<'a, SyncHeatStateEnum>,
    target_temp: temperature::TemperatureProfile,
    pid_use: bool,
    pid_p: f32,
    pid_i: f32,
    pid_d: f32,
    controller: Controller,
    pwm_config: pwm::Config,
    adc: Adc<'a, embassy_rp::adc::Async>,
    adc_temp_ch: Channel<'a>,
    thermistor: &'a Thermistor,
    mosfet: Pwm<'a, embassy_rp::peripherals::PWM_CH3>,
    display_tx: SyncStateChannelSender<'a, SyncDisplayStateEnum>,
    wd_tx: SyncStateChannelSender<'a, SyncWdStateEnum>,
}

impl<'a> Heater<'a> {
    pub fn new(
        startup_storage: &storage::StorageData,
        adc: Adc<'a, embassy_rp::adc::Async>,
        adc_temp_ch: Channel<'a>,
        thermistor: &'a Thermistor,
        mosfet: Pwm<'a, embassy_rp::peripherals::PWM_CH3>,
        channels: &'a channels::Channels,
    ) -> Self {
        Self {
            channel: channels.get_heat_rx(),
            target_temp: temperature::TemperatureProfile::new(
                0,
                temperature::TemperatureProfileEnum::Static,
                startup_storage.temp_wait_time,
                startup_storage.temp_extra_time,
                startup_storage.temp_lead_offset,
                startup_storage.temp_offset,
            ),
            pid_use: startup_storage.pid,
            pid_p: startup_storage.pid_p,
            pid_i: startup_storage.pid_i,
            pid_d: startup_storage.pid_d,
            controller: Controller::new(
                0 as f64,
                startup_storage.pid_p as f64,
                startup_storage.pid_i as f64,
                startup_storage.pid_d as f64,
            ),
            pwm_config: pwm::Config::default(),
            adc,
            adc_temp_ch,
            thermistor,
            mosfet,
            display_tx: channels.get_display_tx(),
            wd_tx: channels.get_watchdog_tx(),
        }
    }

    pub async fn heat_task(&mut self) -> ! {
        let rx = self.channel;
        let mut time_begin = embassy_time::Instant::now();
        let mut last_temp_target = 0;
        loop {
            //recv updates or sleep
            let recv_fut = rx.receive();
            let sleep_fut = Timer::after_millis(100);
            let select_fut = select!(recv_fut, sleep_fut,);
            match select_fut.await {
                embassy_futures::select::Either::First(state) => match state {
                    SyncHeatStateEnum::TargetTemp(temp, prof) => {
                        self.target_temp.set_profile(prof);
                        self.target_temp.set_peak(temp);
                        self.target_temp.reset();
                        self.controller.reset();
                    }
                    SyncHeatStateEnum::Pid {
                        pid,
                        pid_p,
                        pid_i,
                        pid_d,
                    } => {
                        self.pid_use = pid;
                        self.pid_p = pid_p;
                        self.pid_i = pid_i;
                        self.pid_d = pid_d;
                        self.controller.set_proportional_gain(self.pid_p as f64);
                        self.controller.set_integral_gain(self.pid_i as f64);
                        self.controller.set_derivative_gain(self.pid_d as f64);
                        self.controller.reset();
                    }
                    SyncHeatStateEnum::TempSettings {
                        wait_time,
                        extra_time,
                        temp_lead_offset,
                        temp_offset,
                    } => {
                        self.target_temp.set_settings(
                            wait_time,
                            extra_time,
                            temp_lead_offset,
                            temp_offset,
                        );
                    }
                },
                embassy_futures::select::Either::Second(()) => {}
            }

            //read current temp
            let temp_val = self
                .adc
                .read(&mut self.adc_temp_ch)
                .await
                .expect("heat_task: temp fail");
            let current_temp = self.thermistor.calc_temp(temp_val) as u16;

            let time_elapsed = embassy_time::Instant::now() - time_begin;
            if time_elapsed.as_millis() > 10 {
                //calc corrections
                self.target_temp.update(time_elapsed.into(), current_temp);
                let current_temp_target = self.target_temp.get_current_target();
                self.pwm_config.compare_a = if !self.pid_use {
                    if current_temp < current_temp_target {
                        self.pwm_config.top
                    } else {
                        0
                    }
                } else {
                    if current_temp_target != last_temp_target {
                        last_temp_target = current_temp_target;
                        self.controller.set_target(current_temp_target as f64);
                        self.controller.reset();
                    }
                    self.controller
                        .update_elapsed(current_temp as f64, time_elapsed.into())
                        as u16
                };

                //set mosfet
                self.mosfet.set_config(&self.pwm_config);

                time_begin = embassy_time::Instant::now();

                //send updates
                if self
                    .display_tx
                    .try_send(SyncDisplayStateEnum::CurrTemp(current_temp))
                    .is_err()
                {
                    //ignore: msg dropped
                }
                if self
                    .display_tx
                    .try_send(SyncDisplayStateEnum::CurrTargetTemp(current_temp_target))
                    .is_err()
                {
                    //ignore: msg dropped
                }
                if self
                    .display_tx
                    .try_send(SyncDisplayStateEnum::OutputEnabled(
                        self.pwm_config.compare_a > 0,
                    ))
                    .is_err()
                {
                    //ignore: msg dropped
                }
            }

            //feed wd
            self.wd_tx
                .try_send(SyncWdStateEnum::HeatTask)
                .expect("heat_task: wdtx fail");
        }
    }
}
