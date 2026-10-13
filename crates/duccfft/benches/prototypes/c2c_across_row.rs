// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

//! Experimental power-of-two C2C path with SIMD lanes across independent rows.

use num_complex::Complex64;

pub struct AcrossRowPlan {
    len: usize,
    roots: Vec<Complex64>,
    bit_reversed: Vec<usize>,
}

pub struct AcrossRowWorkspace {
    rows: usize,
    values: Vec<Complex64>,
}

impl AcrossRowPlan {
    pub fn new(len: usize) -> Result<Self, &'static str> {
        if len == 0 || !len.is_power_of_two() {
            return Err("prototype length must be a positive power of two");
        }

        let bits = len.trailing_zeros();
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(len)
            .map_err(|_| "prototype plan allocation failed")?;
        for index in 0..len {
            let angle = -std::f64::consts::TAU * index as f64 / len as f64;
            roots.push(Complex64::new(angle.cos(), angle.sin()));
        }
        let mut bit_reversed = Vec::new();
        bit_reversed
            .try_reserve_exact(len)
            .map_err(|_| "prototype plan allocation failed")?;
        for index in 0..len {
            bit_reversed.push(if bits == 0 {
                0
            } else {
                index.reverse_bits() >> (usize::BITS - bits)
            });
        }
        Ok(Self {
            len,
            roots,
            bit_reversed,
        })
    }

    pub fn workspace(&self, rows: usize) -> Result<AcrossRowWorkspace, &'static str> {
        let element_count = self
            .len
            .checked_mul(rows)
            .ok_or("batch element count overflow")?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(element_count)
            .map_err(|_| "prototype workspace allocation failed")?;
        values.resize(element_count, Complex64::new(0.0, 0.0));
        Ok(AcrossRowWorkspace { rows, values })
    }

    pub fn execute(
        &self,
        values: &mut [Complex64],
        workspace: &mut AcrossRowWorkspace,
        inverse: bool,
    ) -> Result<(), &'static str> {
        self.execute_with_backend(values, workspace, inverse, avx2_available())
    }

    pub fn execute_scalar(
        &self,
        values: &mut [Complex64],
        workspace: &mut AcrossRowWorkspace,
        inverse: bool,
    ) -> Result<(), &'static str> {
        self.execute_with_backend(values, workspace, inverse, false)
    }

    fn execute_with_backend(
        &self,
        values: &mut [Complex64],
        workspace: &mut AcrossRowWorkspace,
        inverse: bool,
        use_avx2: bool,
    ) -> Result<(), &'static str> {
        if !values.len().is_multiple_of(self.len) {
            return Err("batch buffer has an incomplete row");
        }
        let rows = values.len() / self.len;
        if workspace.rows != rows || workspace.values.len() != values.len() {
            return Err("workspace row count does not match batch");
        }

        for (row, source) in values.chunks_exact(self.len).enumerate() {
            for (index, &value) in source.iter().enumerate() {
                workspace.values[self.bit_reversed[index] * rows + row] = value;
            }
        }

        if use_avx2 {
            #[cfg(target_arch = "x86_64")]
            // SAFETY: runtime dispatch confirms AVX2. The kernel writes only
            // disjoint radix-two butterfly pairs inside the workspace.
            unsafe {
                stage_avx2(&mut workspace.values, self.len, rows, &self.roots, inverse);
                if inverse {
                    scale_avx2(&mut workspace.values, self.len);
                }
            }
            #[cfg(not(target_arch = "x86_64"))]
            stage_scalar(&mut workspace.values, self.len, rows, &self.roots, inverse);
        } else {
            stage_scalar(&mut workspace.values, self.len, rows, &self.roots, inverse);
        }

        for (row, destination) in values.chunks_exact_mut(self.len).enumerate() {
            for (index, slot) in destination.iter_mut().enumerate() {
                *slot = workspace.values[index * rows + row];
            }
        }
        Ok(())
    }
}

impl AcrossRowWorkspace {
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn scratch_bytes(&self) -> usize {
        self.values.len() * std::mem::size_of::<Complex64>()
    }
}

fn stage_scalar(
    values: &mut [Complex64],
    len: usize,
    rows: usize,
    roots: &[Complex64],
    inverse: bool,
) {
    let mut width = 2;
    while width <= len {
        let half = width / 2;
        let root_stride = len / width;
        for base in (0..len).step_by(width) {
            for inner in 0..half {
                let root = roots[inner * root_stride];
                let twiddle = if inverse { root.conj() } else { root };
                for row in 0..rows {
                    let left_index = (base + inner) * rows + row;
                    let right_index = (base + inner + half) * rows + row;
                    let left = values[left_index];
                    let right = values[right_index] * twiddle;
                    values[left_index] = left + right;
                    values[right_index] = left - right;
                }
            }
        }
        if width == len {
            break;
        }
        width *= 2;
    }

    if inverse {
        let scale = 1.0 / len as f64;
        for value in values {
            *value *= scale;
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn avx2_available() -> bool {
    std::is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
fn avx2_available() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn stage_avx2(
    values: &mut [Complex64],
    len: usize,
    rows: usize,
    roots: &[Complex64],
    inverse: bool,
) {
    use std::arch::x86_64::*;

    let pointer = values.as_mut_ptr();
    let mut width = 2;
    while width <= len {
        let half = width / 2;
        let root_stride = len / width;
        for base in (0..len).step_by(width) {
            for inner in 0..half {
                let root = roots[inner * root_stride];
                let scalar_twiddle = if inverse { root.conj() } else { root };
                let twiddle = _mm256_setr_pd(
                    scalar_twiddle.re,
                    scalar_twiddle.im,
                    scalar_twiddle.re,
                    scalar_twiddle.im,
                );
                let repeated_real = _mm256_movedup_pd(twiddle);
                let repeated_imaginary = _mm256_permute_pd(twiddle, 0b1111);
                let left_index = (base + inner) * rows;
                let right_index = (base + inner + half) * rows;
                let mut row = 0;
                while row + 1 < rows {
                    let left = unsafe { _mm256_loadu_pd(pointer.add(left_index + row).cast()) };
                    let right = unsafe { _mm256_loadu_pd(pointer.add(right_index + row).cast()) };
                    let swapped = _mm256_permute_pd(right, 0b0101);
                    let product = _mm256_addsub_pd(
                        _mm256_mul_pd(right, repeated_real),
                        _mm256_mul_pd(swapped, repeated_imaginary),
                    );
                    unsafe {
                        _mm256_storeu_pd(
                            pointer.add(left_index + row).cast(),
                            _mm256_add_pd(left, product),
                        );
                        _mm256_storeu_pd(
                            pointer.add(right_index + row).cast(),
                            _mm256_sub_pd(left, product),
                        );
                    }
                    row += 2;
                }
                if row < rows {
                    let left_index = left_index + row;
                    let right_index = right_index + row;
                    let left = unsafe { *pointer.add(left_index) };
                    let right = unsafe { *pointer.add(right_index) };
                    let product = right * scalar_twiddle;
                    unsafe {
                        *pointer.add(left_index) = left + product;
                        *pointer.add(right_index) = left - product;
                    }
                }
            }
        }
        if width == len {
            break;
        }
        width *= 2;
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn scale_avx2(values: &mut [Complex64], len: usize) {
    use std::arch::x86_64::*;

    let pointer = values.as_mut_ptr();
    let scale = _mm256_set1_pd(1.0 / len as f64);
    let pairs = values.len() / 2;
    for pair in 0..pairs {
        let value = unsafe { _mm256_loadu_pd(pointer.add(pair * 2).cast()) };
        unsafe { _mm256_storeu_pd(pointer.add(pair * 2).cast(), _mm256_mul_pd(value, scale)) };
    }
    if !values.len().is_multiple_of(2) {
        unsafe { *pointer.add(values.len() - 1) *= 1.0 / len as f64 };
    }
}
