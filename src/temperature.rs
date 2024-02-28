use core::time::Duration;

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TemperatureProfileEnum {
    Static = 0,
    ProfileA = 1,
}

enum TemperatureProfileState {
    FirstRamp,
    FirstRampSync,
    PreHeat,
    SecondRamp,
    SecondRampSync,
    PeakRamp,
    PeakRampSync,
    Cooldown,
}

pub struct TemperatureProfile {
    peak: u16,
    profile: TemperatureProfileEnum,
    time: f32,
    state: TemperatureProfileState,
    state_start: f32,
    temperature: u16,
}

impl TemperatureProfile {
    pub fn new(peak: u16, profile: TemperatureProfileEnum) -> Self {
        Self {
            peak,
            profile,
            time: 0.0,
            state: TemperatureProfileState::FirstRamp,
            state_start: 0.0,
            temperature: 0,
        }
    }

    pub fn set_profile(&mut self, profile: TemperatureProfileEnum) {
        self.profile = profile;
    }

    pub fn set_peak(&mut self, peak: u16) {
        self.peak = peak;
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.state = TemperatureProfileState::FirstRamp;
        self.state_start = 0.0;
        self.temperature = 0;
    }

    pub fn get_current_target(&mut self) -> u16 {
        match self.profile {
            TemperatureProfileEnum::Static => self.get_current_target_static(),
            TemperatureProfileEnum::ProfileA => self.get_current_target_prof_a(),
        }
    }

    fn get_current_target_static(&self) -> u16 {
        self.peak
    }

    fn get_current_target_prof_a(&mut self) -> u16 {
        match self.state {
            TemperatureProfileState::FirstRamp => {
                if self.time >= 38.0 {
                    self.state = TemperatureProfileState::FirstRampSync;
                    self.state_start = self.time;
                }
                (self.time * 4.0) as u16 //time: 0..38 => temp: 0..152
            }
            TemperatureProfileState::FirstRampSync => {
                if self.temperature >= 150 {
                    self.state = TemperatureProfileState::PreHeat;
                    self.state_start = self.time;
                }
                150
            }
            TemperatureProfileState::PreHeat => {
                if self.time >= self.state_start + 80.0 {
                    self.state = TemperatureProfileState::SecondRamp;
                    self.state_start = self.time;
                }
                let diff = self.time - self.state_start;
                150 + (diff * 30.0 / 80.0) as u16 //time: 38..120 => temp: 150..180
            }
            TemperatureProfileState::SecondRamp => {
                if self.time >= self.state_start + 13.0 {
                    self.state = TemperatureProfileState::SecondRampSync;
                    self.state_start = self.time;
                }
                let diff = self.time - self.state_start;
                180 + (diff * 40.0 / 13.0) as u16 //time: 120..133 => temp: 180..220
            }
            TemperatureProfileState::SecondRampSync => {
                if self.time >= self.state_start + 10.0 || self.temperature >= 220 {
                    self.state = TemperatureProfileState::PeakRamp;
                    self.state_start = self.time;
                }
                220
            }
            TemperatureProfileState::PeakRamp => {
                if self.time >= self.state_start + 20.0 {
                    self.state = TemperatureProfileState::PeakRampSync;
                    self.state_start = self.time;
                }
                let diff = self.time - self.state_start;
                let temp_diff = self.peak - 220;
                220 + (temp_diff as f32 * diff / 20.0) as u16 //time: 133..153 => temp: 220..peak
            }
            TemperatureProfileState::PeakRampSync => {
                if self.time >= self.state_start + 10.0 || self.temperature >= self.peak {
                    self.state = TemperatureProfileState::Cooldown;
                    self.state_start = self.time;
                }
                self.peak
            }
            TemperatureProfileState::Cooldown => 0,
        }
    }

    pub fn update(&mut self, duration: Duration, curr_temp: u16) {
        self.time += duration.as_millis() as f32 / 1000.0;
        self.temperature = curr_temp;
    }
}
