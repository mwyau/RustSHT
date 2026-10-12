// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::error::ShtError;
use crate::fft::RealFftPlan;
use crate::grid::GridSpec;
use crate::harmonics::HarmonicSpec;
use crate::legendre::ShtScalar;
use crate::meridional::CCResamplingPlan;
use crate::workspace::ShtWorkspace;

/// Reusable grid, harmonic and Fourier metadata for spherical transforms.
#[allow(private_bounds)]
pub struct ShtPlan<T: ShtScalar = f64> {
    pub(crate) grid: GridSpec,
    pub(crate) harmonics: HarmonicSpec,
    pub(crate) cc_resampling: Option<CCResamplingPlan<T>>,
    pub(crate) fft: RealFftPlan<T>,
    pub(crate) sectoral_norms: Vec<T>,
    pub(crate) sample_count: usize,
    pub(crate) spectrum_count: usize,
    pub(crate) recurrence_factor_count: usize,
    pub(crate) spin_recurrence_factor_count: usize,
    pub(crate) spin_fast_coefficient_count: usize,
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    /// Constructs a plan for a grid and harmonic bandlimit.
    ///
    /// The plan is immutable. Each concurrent operation needs its own
    /// workspace, created by [`Self::workspace`].
    ///
    /// # Errors
    ///
    /// Returns an error if checked sample or spectrum sizes overflow, or if
    /// FFT plans, normalization factors, or their storage cannot be created.
    pub fn new(grid: GridSpec, harmonics: HarmonicSpec) -> Result<Self, ShtError> {
        let sample_count =
            grid.ntheta()
                .checked_mul(grid.nphi())
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "grid sample count",
                })?;
        let fft = RealFftPlan::<T>::new(grid.nphi())?;
        let spectrum_count =
            grid.ntheta()
                .checked_mul(fft.spectrum_len())
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "grid Fourier spectrum count",
                })?;
        let recurrence_factor_count =
            harmonics
                .lmax()
                .checked_add(5)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "scalar recurrence factor count",
                })?;
        let spin_recurrence_factor_count =
            harmonics
                .lmax()
                .checked_add(2)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin recurrence factor count",
                })?;
        let spin_fast_coefficient_count =
            if harmonics.spin() == 1 && harmonics.lmax() >= 500 && T::supports_paired_scalar() {
                harmonics
                    .lmax()
                    .checked_add(2)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "spin-1 fast scalar coefficient count",
                    })?
            } else {
                0
            };
        let sectoral_norms = T::sectoral_normalizations(harmonics.mmax())?;
        let cc_resampling = match grid.kind() {
            crate::grid::GridKind::ClenshawCurtis | crate::grid::GridKind::Fejer1 => Some(
                CCResamplingPlan::<T>::new(grid.kind(), grid.ntheta(), harmonics.lmax())?,
            ),
            crate::grid::GridKind::GaussLegendre => None,
        };

        Ok(Self {
            grid,
            harmonics,
            cc_resampling,
            fft,
            sectoral_norms,
            sample_count,
            spectrum_count,
            recurrence_factor_count,
            spin_recurrence_factor_count,
            spin_fast_coefficient_count,
        })
    }

    /// Returns the grid used by this plan.
    #[must_use]
    pub const fn grid(&self) -> &GridSpec {
        &self.grid
    }

    /// Returns the harmonic layout used by this plan.
    #[must_use]
    pub const fn harmonics(&self) -> &HarmonicSpec {
        &self.harmonics
    }

    /// Creates mutable scratch sized for this plan.
    ///
    /// With the `parallel` feature, worker scratch uses the thread count of the
    /// active Rayon pool. Create the workspace inside a custom pool when that
    /// pool will execute the transform.
    ///
    /// # Errors
    ///
    /// Returns an error if any workspace allocation fails.
    pub fn workspace(&self) -> Result<ShtWorkspace<T>, ShtError> {
        ShtWorkspace::<T>::new(
            self.spectrum_count,
            self.grid.nphi(),
            self.grid.ntheta(),
            self.recurrence_factor_count,
            self.spin_recurrence_factor_count,
            self.spin_fast_coefficient_count,
            &self.fft,
            self.cc_resampling.as_ref(),
        )
    }
}
