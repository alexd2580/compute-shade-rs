use std::{f32::consts::PI, ffi::c_void, mem, sync::Arc};

use realfft::{RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex;

pub struct Dft {
    r2c: Arc<dyn RealToComplex<f32>>,

    hamming: Vec<f32>,

    input: Vec<f32>,
    scratch: Vec<Complex<f32>>,
    output: Vec<Complex<f32>>,

    simple: Vec<f32>,
}

impl Dft {
    pub fn output_byte_size(input_size: usize) -> usize {
        (input_size / 2 + 1) * mem::size_of::<f32>()
    }

    pub fn new(length: usize) -> Self {
        let mut real_planner = RealFftPlanner::<f32>::new();
        let r2c = real_planner.plan_fft_forward(length);

        let input = r2c.make_input_vec();
        let scratch = r2c.make_scratch_vec();
        let output = r2c.make_output_vec();

        assert_eq!(input.len(), length);
        // assert_eq!(scratch.len(), length);
        assert_eq!(output.len(), length / 2 + 1);

        let mut hamming = vec![0f32; length];
        for (index, val) in hamming.iter_mut().enumerate() {
            *val = 0.54 - (0.46 * (2f32 * PI * (index as f32 / (length - 1) as f32)).cos());
            // debug!("{}", *val);
        }

        let simple = vec![0.0; length / 2 + 1];

        Dft {
            r2c,
            hamming,
            input,
            scratch,
            output,
            simple,
        }
    }

    pub fn get_input_vec(&mut self) -> &mut [f32] {
        &mut self.input
    }

    pub fn write_to_pointer(&self, target: *mut c_void) {
        unsafe {
            let size = self.simple.len() as u32;
            *target.cast() = size;
            let target = target.add(mem::size_of::<i32>());

            self.simple.as_ptr().copy_to(target.cast(), size as usize);
        }
    }

    pub fn apply_hamming(&mut self) {
        for (val, factor) in self.input.iter_mut().zip(self.hamming.iter()) {
            *val *= factor;
        }
    }

    pub fn run_transform(&mut self) {
        self.r2c
            .process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch)
            .unwrap();

        // Experimentally determined factor, scales the majority of frequencies to [0..1].
        let factor = 1f32 / (0.27 * self.input.len() as f32);
        for (&output, simple) in self.output.iter().zip(self.simple.iter_mut()) {
            let next_val = output.norm() * factor;
            *simple = 0f32.max(*simple - 0.015).max(next_val);
        }
    }
}
