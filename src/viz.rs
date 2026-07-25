use rustfft::{num_complex::Complex, FftPlanner};

pub const BARS: usize = 64;
pub const FFT_N: usize = 1024;

pub fn compute(samples: &[f32]) -> [u16; BARS] {
    let mut input = [Complex::new(0.0, 0.0); FFT_N];
    for (i, &s) in samples.iter().take(FFT_N).enumerate() {
        let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_N as f32).cos());
        input[i] = Complex::new(s * w, 0.0);
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_N);
    fft.process(&mut input);

    let mut out = [0u16; BARS];
    for b in 0..BARS {
        let lo = (b * FFT_N / 2) / BARS;
        let hi = ((b + 1) * FFT_N / 2) / BARS;
        let mut mag = 0.0;
        for k in lo..hi.max(lo + 1) {
            mag += input[k].norm();
        }
        mag /= (hi.max(lo + 1) - lo) as f32;
        let level = (mag * 6.0).tanh();
        out[b] = (level * 20.0) as u16;
    }
    out
}