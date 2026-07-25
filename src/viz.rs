use rustfft::{num_complex::Complex, FftPlanner};

pub const FFT_N: usize = 1024;

/// Compute `num_bars` spectrum magnitudes (0..=20 each) from the given mono samples.
pub fn compute(samples: &[f32], num_bars: usize) -> Vec<u16> {
    let num_bars = num_bars.max(1);
    let mut input = [Complex::new(0.0, 0.0); FFT_N];
    for (i, &s) in samples.iter().take(FFT_N).enumerate() {
        let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_N as f32).cos());
        input[i] = Complex::new(s * w, 0.0);
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_N);
    fft.process(&mut input);

    // Log-frequency binning: low frequencies get more resolution, highs compressed.
    let mut out = vec![0u16; num_bars];
    let max_k = (FFT_N / 2) as f32;
    for b in 0..num_bars {
        let f0 = (b as f32 / num_bars as f32).powi(2);
        let f1 = ((b + 1) as f32 / num_bars as f32).powi(2);
        let lo = (f0 * max_k) as usize;
        let hi = (((f1 * max_k) as usize) + 1).min(FFT_N / 2).max(lo + 1);
        let mut mag = 0.0f32;
        for k in lo..hi {
            mag = mag.max(input[k].norm());
        }
        let level = (mag * 7.0).tanh();
        out[b] = (level * 20.0) as u16;
    }
    out
}