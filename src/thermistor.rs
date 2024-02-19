use micromath::F32Ext;

const KELVIN_TO_CELSIUS: f32 = -273.15;
const R: f32 = 5000.0;
const INLINE_R: f32 = 0.0;
const VAL_MAX: f32 = 4058.0;

pub(crate) struct Thermistor {
    c1: f32,
    c2: f32,
    c3: f32,
}

impl Thermistor {
    fn setup_coefficients(t1: f32, r1: f32, t2: f32, r2: f32, t3: f32, r3: f32) -> (f32, f32, f32) {
        let inv_t1 = 1.0 / (t1 - KELVIN_TO_CELSIUS);
        let inv_t2 = 1.0 / (t2 - KELVIN_TO_CELSIUS);
        let inv_t3 = 1.0 / (t3 - KELVIN_TO_CELSIUS);

        let ln_r1 = r1.ln();
        let ln_r2 = r2.ln();
        let ln_r3 = r3.ln();

        let ln3_r1 = ln_r1.powi(3);
        let ln3_r2 = ln_r2.powi(3);
        let ln3_r3 = ln_r3.powi(3);

        let inv_t12 = inv_t1 - inv_t2;
        let inv_t13 = inv_t1 - inv_t3;
        let ln_r12 = ln_r1 - ln_r2;
        let ln_r13 = ln_r1 - ln_r3;
        let ln3_r12 = ln3_r1 - ln3_r2;
        let ln3_r13 = ln3_r1 - ln3_r3;

        let c3 = (inv_t12 - inv_t13 * ln_r12 / ln_r13) / (ln3_r12 - ln3_r13 * ln_r12 / ln_r13);
        let c2 = (inv_t12 - c3 * ln3_r12) / ln_r12;
        let c1 = inv_t1 - c2 * ln_r1 - c3 * ln3_r1;

        (c1, c2, c3)
    }

    pub(crate) fn calc_temp(&self, val: u16) -> f32 {
        let fval = val as f32;
        let r = R * fval / (VAL_MAX - fval);
        let ln_r = (r - INLINE_R).ln();
        let inv_t = self.c1 + self.c2 * ln_r + self.c3 * ln_r.powi(3);
        1.0 / inv_t + KELVIN_TO_CELSIUS
    }

    pub(crate) fn new(t1: f32, r1: f32, t2: f32, r2: f32, t3: f32, r3: f32) -> Self {
        let c = Self::setup_coefficients(t1, r1, t2, r2, t3, r3);
        Self {
            c1: c.0,
            c2: c.1,
            c3: c.2,
        }
    }

    pub(crate) fn new_dyze500() -> Self {
        Self::new(25.0, 4500000.0, 260.0, 2240.0, 460.0, 125.4)
    }
}
