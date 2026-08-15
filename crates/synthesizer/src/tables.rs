//! Precomputed source tables used by the formant and excitation stages.

use num_traits::ToPrimitive;

const FORMANT_SAMPLES: usize = 1_280;
const SEGMENT_SAMPLES: usize = 320;
const SINE_SAMPLES: usize = 1_024;
const FREQUENCY_SAMPLES: usize = 4_096;

const FORMANT_POINTS: [[i32; 5]; 3] = [
    [280, 450, 800, 350, 270],
    [600, 800, 1_150, 2_000, 2_140],
    [2_240, 2_830, 2_900, 2_800, 2_950],
];

/// Interpolates five formant control points into a source table.
pub fn formant_curve(points: [i32; 5]) -> Vec<f32> {
    let extended = [
        points[0], points[0], points[1], points[2], points[3], points[4], points[4],
    ];
    let mut output = vec![0.0; FORMANT_SAMPLES];
    for (segment, values) in extended.windows(4).enumerate() {
        let [previous, current, next, after_next] = values else {
            continue;
        };
        let coefficient_a = (3 * (current - next) - previous + after_next)
            .to_f32()
            .unwrap_or_default()
            * 0.5;
        let half = (5 * current).midpoint(*after_next);
        let coefficient_b = (2 * next + previous - half).to_f32().unwrap_or_default();
        let coefficient_c = (next - previous).to_f32().unwrap_or_default() * 0.5;
        let start = segment * SEGMENT_SAMPLES;
        for (sample, destination) in output
            .iter_mut()
            .skip(start)
            .take(SEGMENT_SAMPLES)
            .enumerate()
        {
            let time =
                sample.to_f32().unwrap_or_default() / SEGMENT_SAMPLES.to_f32().unwrap_or(1.0);
            *destination = coefficient_a
                .mul_add(time, coefficient_b)
                .mul_add(time, coefficient_c)
                .mul_add(time, current.to_f32().unwrap_or_default());
        }
    }
    output
}

/// Builds the normalized sine lookup table.
pub fn sine_table() -> Vec<f32> {
    (0..SINE_SAMPLES)
        .map(|index| {
            (std::f32::consts::TAU * index.to_f32().unwrap_or_default()
                / SINE_SAMPLES.to_f32().unwrap_or(1.0))
            .sin()
        })
        .collect()
}

/// Builds the three formant lookup tables used by the voice.
pub fn formant_tables() -> [Vec<f32>; 3] {
    FORMANT_POINTS.map(formant_curve)
}

/// Builds the frequency lookup table used by pitch processing.
pub fn frequency_table() -> Vec<f32> {
    (0..FREQUENCY_SAMPLES)
        .map(|index| 8.175_799_f32 * 1.059_463_f32.powf(index.to_f32().unwrap_or_default() / 32.0))
        .collect()
}

/// Builds the overlap-add window for a grain.
pub fn window(sample_rate: f64, length: usize) -> Vec<f32> {
    let mut output = vec![1.0; length];
    let sample_rate = sample_rate.to_f32().unwrap_or(44_100.0);
    let attack_samples = (sample_rate * 0.0018).max(1.0).to_usize().unwrap_or(1);
    for (sample, destination) in output
        .iter_mut()
        .take(attack_samples.min(length))
        .enumerate()
    {
        *destination = (1.0
            - (std::f32::consts::PI * sample.to_f32().unwrap_or_default()
                / attack_samples.to_f32().unwrap_or(1.0))
            .cos())
            * 0.5;
    }
    let tail_start = (sample_rate * 0.013).to_usize().unwrap_or_default();
    let tail_length = (sample_rate * 0.007).max(1.0).to_usize().unwrap_or(1);
    for (sample, destination) in output.iter_mut().enumerate().skip(tail_start.min(length)) {
        *destination = (1.0
            - (std::f32::consts::PI * (sample + tail_length).to_f32().unwrap_or_default()
                / tail_length.to_f32().unwrap_or(1.0))
            .cos())
            * 0.5;
    }
    output
}

/// Builds the deterministic excitation signal for a grain.
pub fn excitation(sample_rate: f64, length: usize) -> Vec<f32> {
    let sample_rate = sample_rate.to_f32().unwrap_or(44_100.0);
    (0..length)
        .map(|sample| {
            let sample = sample.to_f32().unwrap_or_default();
            let time = sample / sample_rate;
            let first = (std::f32::consts::TAU * 4_950.0 * time).sin()
                * (-157.079_64 * 3.0 * sample / sample_rate).exp();
            let second = (std::f32::consts::TAU * 3_800.0 * time).sin()
                * (-157.079_64 * 3.6 * sample / sample_rate).exp();
            first + second
        })
        .collect()
}
