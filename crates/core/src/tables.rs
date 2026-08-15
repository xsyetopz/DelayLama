//! Precomputed source tables used by the formant and excitation stages.

const FORMANT_SAMPLES: usize = 1_280;
const SEGMENT_SAMPLES: usize = 320;
const FORMANT_SEGMENTS: usize = 4;
const SINE_SAMPLES: usize = 1_024;
const FREQUENCY_SAMPLES: usize = 4_096;

const FORMANT_POINTS: [[i32; 5]; 3] = [
    [280, 450, 800, 350, 270],
    [600, 800, 1_150, 2_000, 2_140],
    [2_240, 2_830, 2_900, 2_800, 2_950],
];

pub fn formant_curve(points: [i32; 5]) -> Vec<f32> {
    let extended = [
        points[0], points[0], points[1], points[2], points[3], points[4], points[4],
    ];
    let mut output = vec![0.0; FORMANT_SAMPLES];
    for segment in 0..FORMANT_SEGMENTS {
        let previous = extended[segment];
        let current = extended[segment + 1];
        let next = extended[segment + 2];
        let after_next = extended[segment + 3];
        let coefficient_a = (3 * (current - next) - previous + after_next) as f32 / 2.0;
        let half = ((5 * current) + after_next) / 2;
        let coefficient_b = (2 * next + previous - half) as f32;
        let coefficient_c = (next - previous) as f32 * 0.5;
        for sample in 0..SEGMENT_SAMPLES {
            let time = sample as f32 / SEGMENT_SAMPLES as f32;
            output[segment * SEGMENT_SAMPLES + sample] =
                (((coefficient_a * time + coefficient_b) * time + coefficient_c) * time)
                    + current as f32;
        }
    }
    output
}

pub fn sine_table() -> Vec<f32> {
    (0..SINE_SAMPLES)
        .map(|index| (std::f64::consts::TAU * index as f64 / SINE_SAMPLES as f64).sin() as f32)
        .collect()
}

pub fn formant_tables() -> [Vec<f32>; 3] {
    FORMANT_POINTS.map(formant_curve)
}

pub fn frequency_table() -> Vec<f32> {
    (0..FREQUENCY_SAMPLES)
        .map(|index| (8.175798916_f64 * 1.059463094_f64.powf(index as f64 / 32.0)) as f32)
        .collect()
}

pub fn window(sample_rate: f64, length: usize) -> Vec<f32> {
    let mut output = vec![1.0; length];
    let attack_samples = (sample_rate * 0.0018).max(1.0) as usize;
    for sample in 0..attack_samples.min(length) {
        output[sample] =
            (1.0 - (std::f64::consts::PI * sample as f64 / attack_samples as f64).cos()) as f32
                * 0.5;
    }
    let tail_start = (sample_rate * 0.013) as usize;
    let tail_length = (sample_rate * 0.007).max(1.0) as usize;
    for sample in tail_start.min(length)..length {
        output[sample] = (1.0
            - (std::f64::consts::PI * (sample + tail_length) as f64 / tail_length as f64).cos())
            as f32
            * 0.5;
    }
    output
}

pub fn excitation(sample_rate: f64, length: usize) -> Vec<f32> {
    (0..length)
        .map(|sample| {
            let time = sample as f64 / sample_rate;
            let first = (std::f64::consts::TAU * 4_950.0 * time).sin()
                * (-157.0796327 * 3.0 * sample as f64 / sample_rate).exp();
            let second = (std::f64::consts::TAU * 3_800.0 * time).sin()
                * (-157.0796327 * 3.6 * sample as f64 / sample_rate).exp();
            (first + second) as f32
        })
        .collect()
}
