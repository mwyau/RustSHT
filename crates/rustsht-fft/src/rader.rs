// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Bounded Rader prime passes for the real FFT planner.

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

use crate::FftError;
use crate::complex::{ComplexFftPlan, ComplexFftWorkspace, Direction, try_vec, unity_root};

pub(crate) struct RaderPrimePlan<T> {
    radix: usize,
    convolution_len: usize,
    powers: Vec<usize>,
    inverse_powers: Vec<usize>,
    forward_kernel: Vec<Complex<T>>,
    inverse_kernel: Vec<Complex<T>>,
    kernel_component_max: T,
    convolution: ComplexFftPlan<T>,
    execute: RaderRuntimeExecutor<T>,
}

pub(crate) type RaderRuntimeExecutor<T> = fn(
    &ComplexFftPlan<T>,
    &mut [Complex<T>],
    &mut ComplexFftWorkspace<T>,
    Direction,
) -> Result<(), FftError>;

pub(crate) struct RaderPrimeWorkspace<T> {
    convolution: Vec<Complex<T>>,
    fft: ComplexFftWorkspace<T>,
}

impl<T: Float + FromPrimitive> RaderPrimePlan<T> {
    pub(crate) fn try_new(
        radix: usize,
        execute: RaderRuntimeExecutor<T>,
    ) -> Result<Self, FftError> {
        if !matches!(radix, 241 | 257) {
            return Err(FftError::InvalidLength);
        }
        let convolution_len = radix - 1;
        let generator = primitive_root(radix);
        let mut powers = try_vec(convolution_len, 0usize)?;
        powers[0] = 1;
        for index in 1..convolution_len {
            powers[index] = powers[index - 1] * generator % radix;
        }
        let mut inverse_powers = try_vec(convolution_len, 0usize)?;
        for index in 0..convolution_len {
            inverse_powers[index] = powers[(convolution_len - index) % convolution_len];
        }

        let convolution = ComplexFftPlan::try_new(convolution_len)?;
        let zero = Complex::new(T::zero(), T::zero());
        let mut forward_kernel = try_vec(convolution_len, zero)?;
        let mut inverse_kernel = try_vec(convolution_len, zero)?;
        for index in 0..convolution_len {
            let root = unity_root(inverse_powers[index], radix);
            forward_kernel[index] = root.conj();
            inverse_kernel[index] = root;
        }
        let mut kernel_workspace = convolution.try_workspace()?;
        execute(
            &convolution,
            &mut forward_kernel,
            &mut kernel_workspace,
            Direction::Forward,
        )?;
        execute(
            &convolution,
            &mut inverse_kernel,
            &mut kernel_workspace,
            Direction::Forward,
        )?;
        let kernel_component_max = forward_kernel
            .iter()
            .chain(&inverse_kernel)
            .fold(T::zero(), |maximum, value| {
                maximum.max(value.re.abs()).max(value.im.abs())
            });

        Ok(Self {
            radix,
            convolution_len,
            powers,
            inverse_powers,
            forward_kernel,
            inverse_kernel,
            kernel_component_max,
            convolution,
            execute,
        })
    }

    pub(crate) fn workspace(&self) -> Result<RaderPrimeWorkspace<T>, FftError> {
        Ok(RaderPrimeWorkspace {
            convolution: try_vec(self.convolution_len, Complex::new(T::zero(), T::zero()))?,
            fft: self.convolution.try_workspace()?,
        })
    }

    pub(crate) fn radix(&self) -> usize {
        self.radix
    }

    #[cfg(test)]
    pub(crate) fn m0_pass_report(&self) -> String {
        format!(
            "Rader radix={} convolution={} algorithm=complex-convolution",
            self.radix, self.convolution_len
        )
    }

    #[cfg(test)]
    pub(crate) fn m0_retained_plan_bytes(&self) -> usize {
        self.powers.capacity() * std::mem::size_of::<usize>()
            + self.inverse_powers.capacity() * std::mem::size_of::<usize>()
            + (self.forward_kernel.capacity() + self.inverse_kernel.capacity())
                * std::mem::size_of::<Complex<T>>()
            + self.convolution.m0_retained_plan_bytes()
    }

    pub(crate) fn is_safe_components(
        &self,
        components: impl IntoIterator<Item = T>,
        transform_len: usize,
        pre_rader_growth: usize,
    ) -> bool {
        let mut maximum = T::zero();
        let mut minimum_nonzero = None;
        for value in components {
            if !value.is_finite() {
                return false;
            }
            let magnitude = value.abs();
            maximum = maximum.max(magnitude);
            if magnitude != T::zero() {
                minimum_nonzero =
                    Some(minimum_nonzero.map_or(magnitude, |current: T| current.min(magnitude)));
            }
        }
        if maximum == T::zero() {
            return true;
        }

        // The inner forward FFT has at most (p-1) terms per component. Its
        // product with a pretransformed Rader kernel is bounded by
        // 2*(p-1)*max(kernel component)*input magnitude. Leave a further
        // factor-four headroom before the inverse FFT consumes that product.
        let product_bound = T::from_usize(8 * self.convolution_len * pre_rader_growth)
            .expect("bounded Rader lengths fit f32 and f64")
            * self.kernel_component_max;
        if maximum > T::max_value() / product_bound {
            return false;
        }

        // Rader's convolution can multiply and add values before the outer
        // transform has a chance to apply its scale. Route values near the
        // subnormal range and terms below a conservative summation error
        // bound through the existing residual-aware Bluestein path.
        let subnormal_bound = T::min_positive_value()
            * T::from_usize(4 * self.convolution_len).expect("bounded radix fits f32 and f64");
        let relative_bound = maximum
            * T::epsilon()
            * T::from_usize(16 * transform_len).expect("bounded transform length fits f32 and f64");
        minimum_nonzero.is_none_or(|minimum| minimum >= subnormal_bound.max(relative_bound))
    }

    pub(crate) fn execute(
        &self,
        input: &[Complex<T>],
        output: &mut [Complex<T>],
        direction: Direction,
        workspace: &mut RaderPrimeWorkspace<T>,
    ) {
        debug_assert_eq!(input.len(), self.radix);
        debug_assert_eq!(output.len(), self.radix);
        debug_assert_eq!(workspace.convolution.len(), self.convolution_len);

        let mut sum = input[0];
        for (index, &power) in self.powers.iter().enumerate() {
            let value = input[power];
            workspace.convolution[index] = value;
            sum = sum + value;
        }
        (self.execute)(
            &self.convolution,
            &mut workspace.convolution,
            &mut workspace.fft,
            Direction::Forward,
        )
        .expect("Rader convolution workspace matches its plan");
        let kernel = match direction {
            Direction::Forward => &self.forward_kernel,
            Direction::Inverse => &self.inverse_kernel,
        };
        for (value, &coefficient) in workspace.convolution.iter_mut().zip(kernel) {
            *value = *value * coefficient;
        }
        (self.execute)(
            &self.convolution,
            &mut workspace.convolution,
            &mut workspace.fft,
            Direction::Inverse,
        )
        .expect("Rader convolution workspace matches its plan");

        output[0] = sum;
        for (index, &power) in self.inverse_powers.iter().enumerate() {
            output[power] = input[0] + workspace.convolution[index];
        }
    }
}

impl<T> RaderPrimeWorkspace<T> {
    pub(crate) fn scratch_bytes(&self) -> Option<usize> {
        self.convolution
            .len()
            .checked_mul(std::mem::size_of::<Complex<T>>())?
            .checked_add(self.fft.scratch_bytes()?)
    }
}

fn primitive_root(prime: usize) -> usize {
    let factors: &[usize] = match prime {
        241 => &[2, 3, 5],
        257 => &[2],
        _ => unreachable!("Rader plans are restricted to the selected primes"),
    };
    'candidate: for candidate in 2..prime {
        for &factor in factors {
            if modular_power(candidate, (prime - 1) / factor, prime) == 1 {
                continue 'candidate;
            }
        }
        return candidate;
    }
    unreachable!("prime has a primitive root")
}

fn modular_power(mut base: usize, mut exponent: usize, modulus: usize) -> usize {
    let mut result = 1;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = result * base % modulus;
        }
        base = base * base % modulus;
        exponent >>= 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::RaderPrimePlan;
    use crate::Complex;
    use crate::complex::{ComplexFftPlan, Direction};

    fn direct(input: &[Complex<f64>], inverse: bool) -> Vec<Complex<f64>> {
        let sign = if inverse { 1.0 } else { -1.0 };
        input
            .iter()
            .enumerate()
            .map(|(frequency, _)| {
                input
                    .iter()
                    .enumerate()
                    .fold(Complex::new(0.0, 0.0), |sum, (sample, value)| {
                        let angle = sign * std::f64::consts::TAU * (sample * frequency) as f64
                            / input.len() as f64;
                        sum + *value * Complex::new(angle.cos(), angle.sin())
                    })
            })
            .collect()
    }

    #[test]
    fn rader_prime_passes_match_unscaled_direct_transforms() {
        for execute in [
            ComplexFftPlan::<f64>::execute_runtime,
            ComplexFftPlan::<f64>::execute,
        ] {
            for radix in [241, 257] {
                let plan = RaderPrimePlan::<f64>::try_new(radix, execute).unwrap();
                let mut workspace = plan.workspace().unwrap();
                let input: Vec<_> = (0..radix)
                    .map(|index| {
                        Complex::new(
                            ((index * 31 % 127) as f64 - 63.0) / 71.0,
                            ((index * 19 % 89) as f64 - 44.0) / 93.0,
                        )
                    })
                    .collect();
                for direction in [Direction::Forward, Direction::Inverse] {
                    let mut actual = vec![Complex::new(0.0, 0.0); radix];
                    plan.execute(&input, &mut actual, direction, &mut workspace);
                    let expected = direct(&input, direction == Direction::Inverse);
                    for (actual, expected) in actual.iter().zip(expected) {
                        assert!((*actual - expected).norm() < 2.0e-10);
                    }
                }
            }
        }
    }

    #[test]
    fn rader_bound_rejects_overflow_underflow_dynamic_range_and_nonfinite_inputs() {
        let prime257 =
            RaderPrimePlan::<f64>::try_new(257, ComplexFftPlan::<f64>::execute_runtime).unwrap();
        assert!(prime257.is_safe_components([1.0, -1.0, 0.5], 257, 1));
        assert!(prime257.is_safe_components([1.0, 1.0e-12], 257, 1));
        assert!(!prime257.is_safe_components([f64::MAX], 257, 1));
        assert!(!prime257.is_safe_components([f64::MIN_POSITIVE], 257, 1));
        assert!(!prime257.is_safe_components([f64::from_bits(1)], 257, 1));
        assert!(!prime257.is_safe_components([1.0, 1.0e-300], 257, 1));
        assert!(!prime257.is_safe_components([f64::NAN], 257, 1));
        assert!(!prime257.is_safe_components([f64::INFINITY], 257, 1));

        let prime241 =
            RaderPrimePlan::<f64>::try_new(241, ComplexFftPlan::<f64>::execute_runtime).unwrap();
        assert!(!prime241.is_safe_components([1.0, 1.0e-12], 723, 3));
    }
}
