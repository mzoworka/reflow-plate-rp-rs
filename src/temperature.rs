use core::{f32::consts::PI, fmt::Debug, time::Duration};

use micromath::F32Ext;

use crate::{menu::SyncMenuStateEnum, tools::SyncStateChannelSender};

const RUNAWAY_TARGET_TEMP_THRESHOLD: u16 = 1;
const RUNAWAY_TEMP_THRESHOLD: u16 = 2;
const TUNE_PID_DELTA: u16 = 5;
const RUNAWAY_INTERVAL: f32 = 5.0;
const PID_PARAM_BASE: f32 = 255.0;

#[derive(Clone)]
pub struct Hidden<T>(T);

impl<T> Debug for Hidden<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Hidden").finish()
    }
}

#[derive(Debug, Clone)]
pub enum TemperatureProfileEnum {
    Static,
    ProfileA{state: TemperatureProfileAState},
    AutoCalibrate{state: TemperatureAutoCalibrateState},
}

#[derive(Debug, Clone)]
pub enum TemperatureProfileAState {
    FirstRamp,
    FirstRampSync,
    FirstRampExtra,
    PreHeat,
    PreHeatExtra,
    SecondRamp,
    SecondRampSync,
    SecondRampExtra,
    PeakRamp,
    PeakRampSync,
    PeakRampExtra,
    Cooldown,
}

impl Default for TemperatureProfileAState {
    fn default() -> Self {
        Self::FirstRamp
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemperatureAutoCalibrateState {
    FirstRamp,
    PeakHeating{n: u8},
    PeakCooling{n: u8},
    Cooldown,
}


impl Default for TemperatureAutoCalibrateState {
    fn default() -> Self {
        Self::FirstRamp
    }
}

pub struct TemperatureProfile<'a> {
    peak: u16,
    profile: TemperatureProfileEnum,
    time: f32,
    state_start: f32,
    temperature: u16,
    temp_wait_time: f32,
    temp_extra_time: f32,
    temp_lead_offset: i16,
    temp_offset: i16,
    curr_max_temp: u16,
    last_target: u16,
    last_max: u16,
    last_period: f32,
    temp_drop_peak: u16,
    peaks: [(f32, f32);2],
    menu_tx: SyncStateChannelSender<'a, SyncMenuStateEnum>,
}

impl TemperatureProfileEnum {
    fn reset(&mut self) {
        match self {
            TemperatureProfileEnum::Static => {},
            TemperatureProfileEnum::ProfileA{ state } => {
                *state = TemperatureProfileAState::FirstRamp;
            },
            TemperatureProfileEnum::AutoCalibrate { state } => {
                *state = TemperatureAutoCalibrateState::FirstRamp;
            },
        }
    }
}

impl<'a> TemperatureProfile<'a> {
    pub fn new(
        peak: u16,
        profile: TemperatureProfileEnum,
        temp_wait_time: f32,
        temp_extra_time: f32,
        temp_lead_offset: i16,
        temp_offset: i16,
        menu_tx: SyncStateChannelSender<'a, SyncMenuStateEnum>,
    ) -> Self {
        Self {
            peak,
            profile,
            time: 0.0,
            state_start: 0.0,
            temperature: 0,
            temp_wait_time,
            temp_extra_time,
            temp_lead_offset,
            temp_offset,
            curr_max_temp: 0,
            last_target: 0,
            last_max: 0,
            last_period: 0.0,
            temp_drop_peak: 0,
            peaks: [(0.0, 0.0), (0.0, 0.0)],
            menu_tx,
        }
    }

    pub fn set_settings(
        &mut self,
        temp_wait_time: f32,
        temp_extra_time: f32,
        temp_lead_offset: i16,
        temp_offset: i16,
    ) {
        self.temp_wait_time = temp_wait_time;
        self.temp_extra_time = temp_extra_time;
        self.temp_lead_offset = temp_lead_offset;
        self.temp_offset = temp_offset;
    }

    pub fn set_profile(&mut self, profile: TemperatureProfileEnum) {
        self.profile = profile;
    }

    pub fn set_peak(&mut self, peak: u16) {
        self.peak = peak;
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.profile.reset();
        self.state_start = 0.0;
        self.temperature = 0;
        self.last_target = 0;
        self.last_period = 0.0;
        self.last_max = 0;
        self.curr_max_temp = 0;
    }

    pub async fn get_current_target(&mut self) -> u16 {
        self.last_target = match &self.profile {
            TemperatureProfileEnum::Static => self.get_current_target_static(),
            TemperatureProfileEnum::ProfileA{..} => self.get_current_target_prof_a(),
            TemperatureProfileEnum::AutoCalibrate{..} => self.get_current_autocalibrate().await,
        };

        self.last_target
    }

    fn get_current_target_static(&self) -> u16 {
        self.peak
    }

    fn get_current_target_prof_a(&mut self) -> u16 {
        match &mut self.profile {
            TemperatureProfileEnum::ProfileA{state} => match state {
                TemperatureProfileAState::FirstRamp => {
                    if self.time >= 38.0 {
                        *state = TemperatureProfileAState::FirstRampSync;
                        self.state_start = self.time;
                    }
                    ((self.time * 4.0) as u16)
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset) //time: 0..38 => temp: 0..152
                },
                TemperatureProfileAState::FirstRampSync => {
                    if self.time >= self.state_start + self.temp_wait_time
                        || self.temperature >= 150u16.saturating_add_signed(self.temp_offset)
                    {
                        *state = TemperatureProfileAState::FirstRampExtra;
                        self.state_start = self.time;
                    }
                    150u16
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset)
                },
                TemperatureProfileAState::FirstRampExtra => {
                    if self.time >= self.state_start + self.temp_extra_time {
                        *state = TemperatureProfileAState::PreHeat;
                        self.state_start = self.time;
                    }
                    150u16
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset)
                },
                TemperatureProfileAState::PreHeat => {
                    if self.time >= self.state_start + 80.0 {
                        *state = TemperatureProfileAState::PreHeatExtra;
                        self.state_start = self.time;
                    }
                    let diff = self.time - self.state_start;
                    (150 + (diff * 30.0 / 80.0) as u16)
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset) //time: 38..120 => temp: 150..180
                },
                TemperatureProfileAState::PreHeatExtra => {
                    if self.time >= self.state_start + self.temp_extra_time {
                        *state = TemperatureProfileAState::SecondRamp;
                        self.state_start = self.time;
                    }
                    180u16
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset)
                },
                TemperatureProfileAState::SecondRamp => {
                    if self.time >= self.state_start + 13.0 {
                        *state = TemperatureProfileAState::SecondRampSync;
                        self.state_start = self.time;
                    }
                    let diff = self.time - self.state_start;
                    (180 + (diff * 40.0 / 13.0) as u16)
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset) //time: 120..133 => temp: 180..220
                },
                TemperatureProfileAState::SecondRampSync => {
                    if self.time >= self.state_start + self.temp_wait_time
                        || self.temperature >= 220u16.saturating_add_signed(self.temp_offset)
                    {
                        *state = TemperatureProfileAState::SecondRampExtra;
                        self.state_start = self.time;
                    }
                    220u16.saturating_add_signed(self.temp_lead_offset)
                },
                TemperatureProfileAState::SecondRampExtra => {
                    if self.time >= self.state_start + self.temp_extra_time {
                        *state = TemperatureProfileAState::PeakRamp;
                        self.state_start = self.time;
                    }
                    220u16.saturating_add_signed(self.temp_lead_offset)
                },
                TemperatureProfileAState::PeakRamp => {
                    if self.time >= self.state_start + 20.0 {
                        *state = TemperatureProfileAState::PeakRampSync;
                        self.state_start = self.time;
                    }
                    let diff = self.time - self.state_start;
                    let temp_diff = self.peak
                        - 220u16
                            .saturating_add_signed(self.temp_lead_offset)
                            .saturating_add_signed(self.temp_offset);
                    (220 + (temp_diff as f32 * diff / 20.0) as u16)
                        .saturating_add_signed(self.temp_lead_offset)
                        .saturating_add_signed(self.temp_offset)
                    //time: 133..153 => temp: 220..peak
                },
                TemperatureProfileAState::PeakRampSync => {
                    if self.time >= self.state_start + self.temp_wait_time
                        || self.temperature >= self.peak.saturating_add_signed(self.temp_offset)
                    {
                        *state = TemperatureProfileAState::PeakRampExtra;
                        self.state_start = self.time;
                    }
                    self.peak.saturating_add_signed(self.temp_offset)
                },
                TemperatureProfileAState::PeakRampExtra => {
                    if self.time >= self.state_start + self.temp_extra_time {
                        *state = TemperatureProfileAState::Cooldown;
                        self.state_start = self.time;
                    }
                    self.peak.saturating_add_signed(self.temp_offset)
                },
                TemperatureProfileAState::Cooldown => 0,
            },
            _ => panic!("wrong profile, expected profileA"),
        }
    }

    async fn get_current_autocalibrate(&mut self) -> u16 {
        match &mut self.profile {
            TemperatureProfileEnum::AutoCalibrate { state } => {
                let menu_tx = self.menu_tx;
                match *state {
                TemperatureAutoCalibrateState::FirstRamp => {
                    if self.temperature >= self.peak {
                        self.temp_drop_peak = u16::MAX;
                        *state = TemperatureAutoCalibrateState::PeakCooling{n: 0};
                    }
                    self.peak
                },
                TemperatureAutoCalibrateState::PeakHeating{ n } => {
                    if self.temperature >= self.peak {
                        self.peaks[0] = self.peaks[1];
                        self.peaks[1].0 = self.temp_drop_peak.into();
                        self.peaks[1].1 = self.time;
                        self.temp_drop_peak = 0;
                        let next_state = if n >= 12 {
                            TemperatureAutoCalibrateState::Cooldown
                        } else {
                            TemperatureAutoCalibrateState::PeakCooling{n}
                        };

                        *state = next_state.clone();

                        if n >= 4 {
                            let pid = self.calc_pid();
                            menu_tx.send(
                                SyncMenuStateEnum::PidAutoTune {
                                    iteration: n,
                                    pid_p: pid.0, 
                                    pid_i: pid.1, 
                                    pid_d: pid.2, 
                                    done: next_state == TemperatureAutoCalibrateState::Cooldown,
                                }
                            ).await;
                        }

                    } else if self.temperature < self.temp_drop_peak {
                        self.temp_drop_peak = self.temperature;
                    }
                    self.peak
                },
                TemperatureAutoCalibrateState::PeakCooling{ n } => {
                    if self.temperature <= self.peak - TUNE_PID_DELTA {
                        self.peaks[0] = self.peaks[1];
                        self.peaks[1].0 = self.temp_drop_peak.into();
                        self.peaks[1].1 = self.time;
                        self.temp_drop_peak = u16::MAX;
                        *state = TemperatureAutoCalibrateState::PeakHeating{n: n + 1};
                        if n >= 4 {
                            let pid = self.calc_pid();
                            menu_tx.send(
                                SyncMenuStateEnum::PidAutoTune {
                                    iteration: n,
                                    pid_p: pid.0, 
                                    pid_i: pid.1, 
                                    pid_d: pid.2, 
                                    done: false,
                                }
                            ).await;
                        }
                    } else if self.temperature > self.temp_drop_peak {
                        self.temp_drop_peak = self.temperature;
                    }
                    self.peak - TUNE_PID_DELTA
                },
                TemperatureAutoCalibrateState::Cooldown => 0,
            }},
            _ => panic!("wrong profile, expected Autocalibrate"),
        }
    }

    fn calc_pid(&self) -> (f32, f32, f32) {
        let temp_diff = self.peaks[1].0 - self.peaks[0].0;
        let time_diff = self.peaks[1].1 - self.peaks[0].1;
       
        let amplitude = temp_diff.abs() * 0.5f32;
       
        let ku = 4.0f32 * 1.0f32 / (PI * amplitude);
       
        let tu = time_diff;
        let ti = 0.5f32 * tu;
        let td = 0.125f32 * tu;
       
        let kp = 0.6f32 * ku *  PID_PARAM_BASE;
        let ki = kp / ti;
        let kd = kp * td;

        (kp, ki, kd)
    }

    pub fn update(&mut self, duration: Duration, curr_temp: u16, heating: bool) {
        self.time += duration.as_millis() as f32 / 1000.0;
        self.temperature = curr_temp;
        if self.temperature > self.curr_max_temp {
            self.curr_max_temp =
                ((self.temperature as f32 * 0.9) + (self.curr_max_temp as f32 * 0.1)) as u16;
        }
        if heating && !matches!(self.profile, TemperatureProfileEnum::AutoCalibrate { .. }) {
            self.check_thermal_runaway();
        }
    }

    fn check_thermal_runaway(&mut self) {
        if self.temperature + RUNAWAY_TARGET_TEMP_THRESHOLD < self.last_target {
            if self.time - self.last_period >= RUNAWAY_INTERVAL {
                if self.curr_max_temp < self.last_max + RUNAWAY_TEMP_THRESHOLD {
                    panic!(
                        "Thermal runaway!\n{:03} < {:03}",
                        self.curr_max_temp,
                        self.last_max + RUNAWAY_TEMP_THRESHOLD
                    );
                }
                self.last_period = self.time;
                self.last_max = self.curr_max_temp;
            }
        } else {
            self.last_max = 0;
            self.curr_max_temp = self.last_target;
        }
    }
}
