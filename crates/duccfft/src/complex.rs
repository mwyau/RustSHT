// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (fft.h)
// SPDX-FileCopyrightText: 2019-2026 Max-Planck-Society (unity_roots.h)
// SPDX-FileCopyrightText: 2010-2023 Max-Planck-Society
// SPDX-FileCopyrightText: 2019 Peter Bell
// SPDX-License-Identifier: BSD-3-Clause
//
// The factor order follows DUCC's cfftpass::factorize in fft.h at
// c1ed01b8fda69d2a378190f0915f1a19762dddbf. Execution uses an independent
// recursive decimation-in-time schedule with caller-owned scratch.

use std::fmt;

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

/// Errors returned by FFT plan construction and execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FftError {
    /// The transform length must be positive.
    InvalidLength,
    /// The requested plan or workspace could not be allocated.
    AllocationFailed,
    /// An input, output, or workspace length did not match the plan.
    BufferLength {
        /// The name of the buffer with an invalid length.
        buffer: &'static str,
        /// The length required by the plan.
        expected: usize,
        /// The length supplied by the caller.
        actual: usize,
    },
}

impl fmt::Display for FftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "FFT length must be positive"),
            Self::AllocationFailed => write!(f, "FFT plan or workspace allocation failed"),
            Self::BufferLength {
                buffer,
                expected,
                actual,
            } => write!(
                f,
                "{buffer} length is {actual}; the plan requires {expected}"
            ),
        }
    }
}

impl std::error::Error for FftError {}

/// An immutable one-dimensional complex FFT plan.
///
/// Plans support `f32` and `f64`. Forward transforms use the negative
/// exponential and are unnormalized. Inverse transforms use the positive
/// exponential and include a `1/n` scale. A plan can be shared between
/// threads; each concurrent call needs a separate [`ComplexFftWorkspace`].
pub struct ComplexFftPlan<T> {
    data: ComplexPlanData<T>,
}

struct ComplexPlanData<T> {
    len: usize,
    factors: Vec<usize>,
    roots: Vec<Complex<T>>,
    max_radix: usize,
}

/// Mutable scratch for repeated execution of a [`ComplexFftPlan`].
///
/// A workspace can be reused after each operation completes. It must not be
/// used by concurrent operations at the same time.
pub struct ComplexFftWorkspace<T> {
    len: usize,
    output: Vec<Complex<T>>,
    radix_input: Vec<Complex<T>>,
    radix_output: Vec<Complex<T>>,
}

impl<T> ComplexFftWorkspace<T> {
    /// Returns the transform length for which this workspace was allocated.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if this workspace has no elements.
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl<T: Float + FromPrimitive> ComplexFftPlan<T> {
    fn try_new(len: usize) -> Result<Self, FftError> {
        if len == 0 {
            return Err(FftError::InvalidLength);
        }

        let mut roots = try_vec(len, Complex::new(T::zero(), T::zero()))?;
        let factors = factorize(len)?;
        let max_radix = factors.iter().copied().max().unwrap_or(1);
        for (index, root) in roots.iter_mut().enumerate() {
            *root = unity_root(index, len);
        }

        Ok(Self {
            data: ComplexPlanData {
                len,
                factors,
                roots,
                max_radix,
            },
        })
    }

    fn try_workspace(&self) -> Result<ComplexFftWorkspace<T>, FftError> {
        let zero = Complex::new(T::zero(), T::zero());
        Ok(ComplexFftWorkspace {
            len: self.data.len,
            output: try_vec(self.data.len, zero)?,
            radix_input: try_vec(self.data.max_radix, zero)?,
            radix_output: try_vec(self.data.max_radix, zero)?,
        })
    }

    fn execute(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
    ) -> Result<(), FftError> {
        check_len("values", self.data.len, values.len())?;
        check_len("workspace", self.data.len, workspace.len)?;

        {
            let mut executor = Executor {
                input: values,
                roots: &self.data.roots,
                total_len: self.data.len,
                direction,
                output: &mut workspace.output,
                radix_input: &mut workspace.radix_input,
                radix_output: &mut workspace.radix_output,
            };
            executor.recurse(0, 1, self.data.len, &self.data.factors, 0);
        }

        for (value, transformed) in values.iter_mut().zip(&workspace.output) {
            *value = *transformed;
        }
        Ok(())
    }
}

macro_rules! implement_public_api {
    ($scalar:ty) => {
        impl ComplexFftPlan<$scalar> {
            /// Constructs a plan for a positive transform length.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::InvalidLength`] for zero and
            /// [`FftError::AllocationFailed`] if plan storage cannot be
            /// reserved.
            pub fn new(len: usize) -> Result<Self, FftError> {
                Self::try_new(len)
            }

            /// Returns the transform length.
            pub fn len(&self) -> usize {
                self.data.len
            }

            /// Returns `true` if the plan length is zero. Plans reject zero
            /// lengths, so this always returns `false`.
            pub fn is_empty(&self) -> bool {
                false
            }

            /// Allocates reusable execution scratch for this plan.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::AllocationFailed`] if scratch storage
            /// cannot be reserved.
            pub fn workspace(&self) -> Result<ComplexFftWorkspace<$scalar>, FftError> {
                self.try_workspace()
            }

            /// Executes an unnormalized forward complex transform in place.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BufferLength`] if `values` or `workspace`
            /// has a length different from this plan.
            pub fn forward(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                self.execute(values, workspace, Direction::Forward)
            }

            /// Executes an inverse complex transform in place and scales the
            /// result by `1/n`.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BufferLength`] if `values` or `workspace`
            /// has a length different from this plan.
            pub fn inverse(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                self.execute(values, workspace, Direction::Inverse)
            }
        }
    };
}

implement_public_api!(f32);
implement_public_api!(f64);

#[derive(Clone, Copy, Eq, PartialEq)]
enum Direction {
    Forward,
    Inverse,
}

struct Executor<'a, T> {
    input: &'a [Complex<T>],
    roots: &'a [Complex<T>],
    total_len: usize,
    direction: Direction,
    output: &'a mut [Complex<T>],
    radix_input: &'a mut [Complex<T>],
    radix_output: &'a mut [Complex<T>],
}

impl<T: Float + FromPrimitive> Executor<'_, T> {
    fn recurse(
        &mut self,
        input_offset: usize,
        input_stride: usize,
        len: usize,
        factors: &[usize],
        output_offset: usize,
    ) {
        if len == 1 {
            self.output[output_offset] = self.input[input_offset];
            return;
        }

        let radix = factors[0];
        let child_len = len / radix;
        for branch in 0..radix {
            self.recurse(
                input_offset + branch * input_stride,
                input_stride * radix,
                child_len,
                &factors[1..],
                output_offset + branch * child_len,
            );
        }

        let stage_root_stride = self.total_len / len;
        let radix_root_stride = self.total_len / radix;
        let inverse = self.direction == Direction::Inverse;
        let stage_scale = if inverse {
            inverse_scale::<T>(radix)
        } else {
            T::one()
        };
        let radix_value = scalar::<T>(radix as f64);
        let overflow_threshold = if inverse {
            T::max_value() / scalar::<T>(4.0 * radix as f64)
        } else {
            T::max_value()
        };
        for frequency in 0..child_len {
            let prescale = if inverse {
                let mut max_component = T::zero();
                for branch in 0..radix {
                    let value = self.output[output_offset + branch * child_len + frequency];
                    max_component = max_component.max(value.re.abs()).max(value.im.abs());
                }
                max_component > overflow_threshold
            } else {
                false
            };
            for branch in 0..radix {
                let mut value = self.output[output_offset + branch * child_len + frequency];
                if prescale {
                    value = value * stage_scale;
                }
                if branch != 0 && frequency != 0 {
                    // branch*frequency < len, so this product is below
                    // total_len and cannot overflow for a valid plan.
                    let root_index = branch * frequency * stage_root_stride;
                    value = value * directed_root(self.roots, root_index, self.direction);
                }
                self.radix_input[branch] = value;
            }

            radix_transform(
                radix,
                &self.radix_input[..radix],
                &mut self.radix_output[..radix],
                self.roots,
                radix_root_stride,
                self.direction,
            );

            if prescale {
                // Recover contributions rounded away while scaling the inputs.
                // The correction is transformed before applying 1/radix so
                // several subnormal terms can combine into a representable
                // result. Ignore a non-finite reconstruction near MAX; that
                // component's residual is below one ulp after normalization.
                for branch in 0..radix {
                    let original = self.output[output_offset + branch * child_len + frequency];
                    let scaled = original * stage_scale;
                    let residual = Complex::new(
                        prescale_residual(original.re, scaled.re, radix_value),
                        prescale_residual(original.im, scaled.im, radix_value),
                    );
                    let residual = if branch != 0 && frequency != 0 {
                        let root_index = branch * frequency * stage_root_stride;
                        residual * directed_root(self.roots, root_index, self.direction)
                    } else {
                        residual
                    };
                    self.radix_input[branch] = residual;
                }
                for branch in 0..radix {
                    self.output[output_offset + frequency + branch * child_len] =
                        self.radix_output[branch];
                }
                radix_transform(
                    radix,
                    &self.radix_input[..radix],
                    &mut self.radix_output[..radix],
                    self.roots,
                    radix_root_stride,
                    self.direction,
                );
            }

            for branch in 0..radix {
                let mut value = self.radix_output[branch];
                if prescale {
                    value = self.output[output_offset + frequency + branch * child_len]
                        + value * stage_scale;
                } else if inverse {
                    value = value * stage_scale;
                }
                self.output[output_offset + frequency + branch * child_len] = value;
            }
        }
    }
}

fn prescale_residual<T: Float>(original: T, scaled: T, radix: T) -> T {
    let reconstructed = scaled * radix;
    if reconstructed.is_finite() {
        original - reconstructed
    } else {
        T::zero()
    }
}

fn radix_transform<T: Float + FromPrimitive>(
    radix: usize,
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    match radix {
        2 => {
            output[0] = input[0] + input[1];
            output[1] = input[0] - input[1];
        }
        3 => radix3(input, output, direction),
        4 => radix4(input, output, direction),
        5 => radix5(input, output, direction),
        7 => radix7(input, output, direction),
        8 => radix8(input, output, direction),
        11 => radix11(input, output, direction),
        _ => radix_direct(radix, input, output, roots, root_stride, direction),
    }
}

fn radix3<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let pair = input[1] + input[2];
    let difference = input[1] - input[2];
    let half = scalar::<T>(0.5);
    let root_three_half = scalar::<T>(0.866_025_403_784_438_6);
    let turn = imaginary(difference) * (root_three_half * direction_sign::<T>(direction));
    let base = input[0] - pair * half;
    output[0] = input[0] + pair;
    output[1] = base + turn;
    output[2] = base - turn;
}

fn radix4<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let even_sum = input[0] + input[2];
    let even_difference = input[0] - input[2];
    let odd_sum = input[1] + input[3];
    let odd_difference = input[1] - input[3];
    output[0] = even_sum + odd_sum;
    output[2] = even_sum - odd_sum;
    output[1] = even_difference + imaginary(odd_difference) * direction_sign::<T>(direction);
    output[3] = even_difference - imaginary(odd_difference) * direction_sign::<T>(direction);
}

fn radix5<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let pair_a = input[1] + input[4];
    let diff_a = input[1] - input[4];
    let pair_b = input[2] + input[3];
    let diff_b = input[2] - input[3];
    let c72 = scalar::<T>(0.309_016_994_374_947_45);
    let c144 = scalar::<T>(-0.809_016_994_374_947_5);
    let s72 = scalar::<T>(0.951_056_516_295_153_5);
    let s144 = scalar::<T>(0.587_785_252_292_473_1);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + pair_a + pair_b;

    let base_one = input[0] + pair_a * c72 + pair_b * c144;
    let turn_one = imaginary(diff_a * s72 + diff_b * s144) * sign;
    output[1] = base_one + turn_one;
    output[4] = base_one - turn_one;

    let base_two = input[0] + pair_a * c144 + pair_b * c72;
    let turn_two = imaginary(diff_a * s144 - diff_b * s72) * sign;
    output[2] = base_two + turn_two;
    output[3] = base_two - turn_two;
}

fn radix7<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let p1 = input[1] + input[6];
    let p2 = input[2] + input[5];
    let p3 = input[3] + input[4];
    let d1 = input[1] - input[6];
    let d2 = input[2] - input[5];
    let d3 = input[3] - input[4];
    let c1 = scalar::<T>(0.623_489_801_858_733_5);
    let c2 = scalar::<T>(-0.222_520_933_956_314_4);
    let c3 = scalar::<T>(-0.900_968_867_902_419_1);
    let s1 = scalar::<T>(0.781_831_482_468_029_8);
    let s2 = scalar::<T>(0.974_927_912_181_823_6);
    let s3 = scalar::<T>(0.433_883_739_117_558_1);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + p1 + p2 + p3;
    let base1 = input[0] + p1 * c1 + p2 * c2 + p3 * c3;
    let turn1 = imaginary(d1 * s1 + d2 * s2 + d3 * s3) * sign;
    output[1] = base1 + turn1;
    output[6] = base1 - turn1;
    let base2 = input[0] + p1 * c2 + p2 * c3 + p3 * c1;
    let turn2 = imaginary(d1 * s2 - d2 * s3 - d3 * s1) * sign;
    output[2] = base2 + turn2;
    output[5] = base2 - turn2;
    let base3 = input[0] + p1 * c3 + p2 * c1 + p3 * c2;
    let turn3 = imaginary(d1 * s3 - d2 * s1 + d3 * s2) * sign;
    output[3] = base3 + turn3;
    output[4] = base3 - turn3;
}

fn radix8<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let mut a1 = input[1] + input[5];
    let mut a5 = input[1] - input[5];
    let mut a3 = input[3] + input[7];
    let mut a7 = input[3] - input[7];
    let old_a1 = a1;
    a1 = a1 + a3;
    a3 = rotate_quarter(old_a1 - a3, direction);
    a7 = rotate_quarter(a7, direction);
    let old_a5 = a5;
    a5 = a5 + a7;
    a7 = rotate_eighth_three(old_a5 - a7, direction);
    a5 = rotate_eighth(a5, direction);

    let a0 = input[0] + input[4];
    let a4 = input[0] - input[4];
    let a2 = input[2] + input[6];
    let mut a6 = input[2] - input[6];
    output[0] = a0 + a2 + a1;
    output[4] = a0 + a2 - a1;
    output[2] = a0 - a2 + a3;
    output[6] = a0 - a2 - a3;
    a6 = rotate_quarter(a6, direction);
    output[1] = a4 + a6 + a5;
    output[5] = a4 + a6 - a5;
    output[3] = a4 - a6 + a7;
    output[7] = a4 - a6 - a7;
}

fn radix11<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let p1 = input[1] + input[10];
    let p2 = input[2] + input[9];
    let p3 = input[3] + input[8];
    let p4 = input[4] + input[7];
    let p5 = input[5] + input[6];
    let d1 = input[1] - input[10];
    let d2 = input[2] - input[9];
    let d3 = input[3] - input[8];
    let d4 = input[4] - input[7];
    let d5 = input[5] - input[6];
    let c1 = scalar::<T>(0.841_253_532_831_181_2);
    let c2 = scalar::<T>(0.415_415_013_001_886_4);
    let c3 = scalar::<T>(-0.142_314_838_273_285_14);
    let c4 = scalar::<T>(-0.654_860_733_945_285_1);
    let c5 = scalar::<T>(-0.959_492_973_614_497_4);
    let s1 = scalar::<T>(0.540_640_817_455_597_6);
    let s2 = scalar::<T>(0.909_631_995_354_518_4);
    let s3 = scalar::<T>(0.989_821_441_880_932_7);
    let s4 = scalar::<T>(0.755_749_574_354_258_3);
    let s5 = scalar::<T>(0.281_732_556_841_429_7);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + p1 + p2 + p3 + p4 + p5;
    let base1 = input[0] + p1 * c1 + p2 * c2 + p3 * c3 + p4 * c4 + p5 * c5;
    let turn1 = imaginary(d1 * s1 + d2 * s2 + d3 * s3 + d4 * s4 + d5 * s5) * sign;
    output[1] = base1 + turn1;
    output[10] = base1 - turn1;
    let base2 = input[0] + p1 * c2 + p2 * c4 + p3 * c5 + p4 * c3 + p5 * c1;
    let turn2 = imaginary(d1 * s2 + d2 * s4 - d3 * s5 - d4 * s3 - d5 * s1) * sign;
    output[2] = base2 + turn2;
    output[9] = base2 - turn2;
    let base3 = input[0] + p1 * c3 + p2 * c5 + p3 * c2 + p4 * c1 + p5 * c4;
    let turn3 = imaginary(d1 * s3 - d2 * s5 - d3 * s2 + d4 * s1 + d5 * s4) * sign;
    output[3] = base3 + turn3;
    output[8] = base3 - turn3;
    let base4 = input[0] + p1 * c4 + p2 * c3 + p3 * c1 + p4 * c5 + p5 * c2;
    let turn4 = imaginary(d1 * s4 - d2 * s3 + d3 * s1 + d4 * s5 - d5 * s2) * sign;
    output[4] = base4 + turn4;
    output[7] = base4 - turn4;
    let base5 = input[0] + p1 * c5 + p2 * c1 + p3 * c4 + p4 * c2 + p5 * c3;
    let turn5 = imaginary(d1 * s5 - d2 * s1 + d3 * s4 - d4 * s2 + d5 * s3) * sign;
    output[5] = base5 + turn5;
    output[6] = base5 - turn5;
}

fn radix_direct<T: Float + FromPrimitive>(
    radix: usize,
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    for (frequency, result) in output.iter_mut().enumerate().take(radix) {
        let mut sum = Complex::new(T::zero(), T::zero());
        for (branch, &value) in input.iter().enumerate().take(radix) {
            let root_index = mul_mod(branch, frequency, radix) * root_stride;
            sum = sum + value * directed_root(roots, root_index, direction);
        }
        *result = sum;
    }
}

fn factorize(len: usize) -> Result<Vec<usize>, FftError> {
    let mut factors = Vec::new();
    factors
        .try_reserve_exact(usize::BITS as usize)
        .map_err(|_| FftError::AllocationFailed)?;

    let mut remaining = len;
    while remaining.is_multiple_of(8) {
        factors.push(8);
        remaining /= 8;
    }
    while remaining.is_multiple_of(4) {
        factors.push(4);
        remaining /= 4;
    }
    if remaining.is_multiple_of(2) {
        remaining /= 2;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }

    let mut divisor = 3;
    while divisor <= remaining / divisor {
        while remaining.is_multiple_of(divisor) {
            factors.push(divisor);
            remaining /= divisor;
        }
        divisor += 2;
    }
    if remaining > 1 {
        factors.push(remaining);
    }
    Ok(factors)
}

fn unity_root<T: Float + FromPrimitive>(index: usize, len: usize) -> Complex<T> {
    if index == 0 {
        return Complex::new(T::one(), T::zero());
    }

    let mut octant_index = index as u128 * 8;
    let n = len as u128;
    let eight_n = 8 * n;
    let (angle_index, cosine_first, real_sign, imag_sign) = if octant_index < 4 * n {
        if octant_index < 2 * n {
            if octant_index < n {
                (octant_index, true, 1.0, 1.0)
            } else {
                (2 * n - octant_index, false, 1.0, 1.0)
            }
        } else {
            octant_index -= 2 * n;
            if octant_index < n {
                (octant_index, false, -1.0, 1.0)
            } else {
                (2 * n - octant_index, true, -1.0, 1.0)
            }
        }
    } else {
        octant_index = eight_n - octant_index;
        if octant_index < 2 * n {
            if octant_index < n {
                (octant_index, true, 1.0, -1.0)
            } else {
                (2 * n - octant_index, false, 1.0, -1.0)
            }
        } else {
            octant_index -= 2 * n;
            if octant_index < n {
                (octant_index, false, -1.0, -1.0)
            } else {
                (2 * n - octant_index, true, -1.0, -1.0)
            }
        }
    };
    let angle = angle_index as f64 * (std::f64::consts::FRAC_PI_4 / len as f64);
    let (sin, cos) = angle.sin_cos();
    let (real_value, imag_value) = if cosine_first { (cos, sin) } else { (sin, cos) };
    let real = real_value * real_sign;
    let imag = imag_value * imag_sign;
    Complex::new(scalar(real), scalar(imag))
}

fn directed_root<T: Float>(roots: &[Complex<T>], index: usize, direction: Direction) -> Complex<T> {
    match direction {
        Direction::Forward => roots[index].conj(),
        Direction::Inverse => roots[index],
    }
}

fn direction_sign<T: Float>(direction: Direction) -> T {
    match direction {
        Direction::Forward => -T::one(),
        Direction::Inverse => T::one(),
    }
}

fn inverse_scale<T: Float + FromPrimitive>(radix: usize) -> T {
    T::from_f64(1.0 / radix as f64).expect("f32 and f64 can represent a positive reciprocal")
}

fn rotate_quarter<T: Float>(value: Complex<T>, direction: Direction) -> Complex<T> {
    match direction {
        Direction::Forward => Complex::new(value.im, -value.re),
        Direction::Inverse => Complex::new(-value.im, value.re),
    }
}

fn rotate_eighth<T: Float + FromPrimitive>(value: Complex<T>, direction: Direction) -> Complex<T> {
    let c = scalar::<T>(std::f64::consts::FRAC_1_SQRT_2);
    let sign = direction_sign::<T>(direction);
    Complex::new(
        (value.re - sign * value.im) * c,
        (sign * value.re + value.im) * c,
    )
}

fn rotate_eighth_three<T: Float + FromPrimitive>(
    value: Complex<T>,
    direction: Direction,
) -> Complex<T> {
    let c = scalar::<T>(std::f64::consts::FRAC_1_SQRT_2);
    match direction {
        Direction::Forward => Complex::new((value.im - value.re) * c, (-value.re - value.im) * c),
        Direction::Inverse => Complex::new((-value.re - value.im) * c, (value.re - value.im) * c),
    }
}

fn imaginary<T: Float>(value: Complex<T>) -> Complex<T> {
    Complex::new(-value.im, value.re)
}

fn scalar<T: FromPrimitive>(value: f64) -> T {
    T::from_f64(value).expect("f32 and f64 represent FFT constants")
}

fn mul_mod(left: usize, right: usize, modulus: usize) -> usize {
    ((left as u128 * right as u128) % modulus as u128) as usize
}

fn try_vec<T: Copy>(len: usize, value: T) -> Result<Vec<T>, FftError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| FftError::AllocationFailed)?;
    result.resize(len, value);
    Ok(result)
}

fn check_len(buffer: &'static str, expected: usize, actual: usize) -> Result<(), FftError> {
    if expected == actual {
        Ok(())
    } else {
        Err(FftError::BufferLength {
            buffer,
            expected,
            actual,
        })
    }
}
