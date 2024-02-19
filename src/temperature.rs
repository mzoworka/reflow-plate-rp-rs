use core::time::Duration;

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TemperatureProfileEnum {
    Static = 0,
    ProfileA = 1,
}

pub struct TemperatureProfile {
    peak: u16,
    profile: TemperatureProfileEnum,
    time: f32,
}

impl TemperatureProfile {
    pub fn new(peak: u16, profile: TemperatureProfileEnum) -> Self {
        Self {
            peak,
            profile,
            time: 0.0,
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
    }

    pub fn get_current_target(&self) -> u16 {
        match self.profile {
            TemperatureProfileEnum::Static => self.peak,
            TemperatureProfileEnum::ProfileA => match self.time as u16 {
                153.. => 0, //cooldown
                133.. => {
                    //peak
                    let diff = self.time - 133.0;
                    let temp_diff = self.peak - 220;
                    220 + (temp_diff as f32 * diff / 20.0) as u16
                }
                120.. => {
                    /* second ramp */
                    let diff = self.time - 120.0;
                    180 + (40.0 * diff / 13.0) as u16
                }
                40.. => {
                    /* pre-heat */
                    let diff = self.time - 40.0;
                    150 + (30.0 * diff / 80.0) as u16
                }
                38.. => 150,
                0.. => (self.time * 4.0) as u16, /* first ramp */
            },
        }
    }

    pub fn update(&mut self, duration: Duration) {
        self.time += duration.as_millis() as f32 / 1000.0;
    }
}
