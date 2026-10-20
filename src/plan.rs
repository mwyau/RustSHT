// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::error::ShtError;
use crate::fft::RealFftPlan;
use crate::grid::GridSpec;
use crate::harmonics::HarmonicSpec;
#[cfg(feature = "parallel")]
use crate::legendre::recurrence_scratch_len;
use crate::legendre::{
    LegendreOperation, LegendreStrategy, SPIN1_FAST_MIN_LMAX, ScalarLegendreTable, ShtScalar,
    fill_spin2_factors, measured_table_break_even_calls, select_scalar_legendre_strategy,
};
use crate::meridional::CCResamplingPlan;
use crate::workspace::ShtWorkspace;
use std::sync::OnceLock;

/// Expected STANDARD transform uses grouped by execution direction.
///
/// The counts are calls that can use the compact table in the corresponding
/// execution direction for the plan's spin, grid, mode, and operation. Scalar
/// coefficient-to-map calls include synthesis and adjoint analysis; scalar
/// map-to-coefficient calls include analysis and adjoint synthesis. For
/// spin-1, STANDARD analysis can use the map-to-coefficient table, but
/// adjoint synthesis currently cannot and must not contribute to that count.
/// `None` leaves the corresponding strategy on the existing automatic
/// steady-state policy. Finite counts affect only table choices with measured
/// cold-to-warm break-even data. An unmeasured cold cell uses recurrence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LegendreReuseEstimate {
    coefficient_to_map_calls: Option<usize>,
    map_to_coefficient_calls: Option<usize>,
}

impl LegendreReuseEstimate {
    /// Creates estimates of STANDARD calls through each execution direction.
    ///
    /// Counts include only STANDARD calls that can execute a compact table
    /// for this plan's spin, grid, mode, and operation. Scalar estimates may
    /// include both each operation and its adjoint. Spin-1 adjoint synthesis
    /// cannot use the map-to-coefficient table and must not be counted there.
    /// Counts include the first call. Use `None` when eligible reuse is
    /// unknown. `Some(0)` keeps a fresh plan on recurrence for that direction.
    #[must_use]
    pub const fn new(
        coefficient_to_map_calls: Option<usize>,
        map_to_coefficient_calls: Option<usize>,
    ) -> Self {
        Self {
            coefficient_to_map_calls,
            map_to_coefficient_calls,
        }
    }

    /// Returns the coefficient-to-map execution estimate, if supplied.
    #[must_use]
    pub const fn coefficient_to_map_calls(self) -> Option<usize> {
        self.coefficient_to_map_calls
    }

    /// Returns the map-to-coefficient execution estimate, if supplied.
    #[must_use]
    pub const fn map_to_coefficient_calls(self) -> Option<usize> {
        self.map_to_coefficient_calls
    }

    pub(crate) const fn calls_for(self, operation: LegendreOperation) -> Option<usize> {
        match operation {
            LegendreOperation::Synthesis => self.coefficient_to_map_calls,
            LegendreOperation::Analysis => self.map_to_coefficient_calls,
        }
    }
}

pub(crate) fn current_execution_thread_count() -> usize {
    #[cfg(feature = "parallel")]
    {
        rayon::current_num_threads()
    }
    #[cfg(not(feature = "parallel"))]
    {
        1
    }
}

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
    pub(crate) spin2_fast_f1: Vec<T>,
    pub(crate) spin2_fast_f2: Vec<T>,
    legendre_reuse_estimate: LegendreReuseEstimate,
    pub(crate) scalar_legendre_table: OnceLock<Option<ScalarLegendreTable<T>>>,
    pub(crate) spin1_scalar_legendre_table: OnceLock<Option<ScalarLegendreTable<T>>>,
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
        Self::new_with_legendre_reuse_estimate(grid, harmonics, LegendreReuseEstimate::default())
    }

    /// Constructs a plan with an estimate of its STANDARD transform reuse.
    ///
    /// The estimate changes only compact-table selection. The selected table
    /// remains lazy, the executing Rayon pool is checked for each operation,
    /// and unmeasured cold break-even cells retain recurrence. If another
    /// eligible operation has already initialized the shared table, a
    /// positive estimate can use it where the existing operation policy
    /// selects it. This does not measure calls or change numerical behavior.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::new`].
    pub fn new_with_legendre_reuse_estimate(
        grid: GridSpec,
        harmonics: HarmonicSpec,
        legendre_reuse_estimate: LegendreReuseEstimate,
    ) -> Result<Self, ShtError> {
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
        let spin2_fast_enabled =
            harmonics.spin() == 2 && harmonics.lmax() >= 500 && T::supports_paired_scalar();
        let recurrence_factor_count = harmonics
            .lmax()
            .checked_add(if spin2_fast_enabled { 6 } else { 5 })
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
        let spin_fast_coefficient_count = match (harmonics.spin(), T::supports_paired_scalar()) {
            (0, true) => harmonics
                .lmax()
                .checked_add(2)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "paired scalar coefficient scratch count",
                })?,
            (1, true) if harmonics.lmax() >= SPIN1_FAST_MIN_LMAX => harmonics
                .lmax()
                .checked_add(2)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin-1 fast scalar coefficient count",
                })?,
            (2, true) if harmonics.lmax() >= 500 => {
                harmonics
                    .lmax()
                    .checked_add(5)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "spin-2 fast scalar coefficient count",
                    })?
            }
            _ => 0,
        };
        let spin2_factor_count = if spin2_fast_enabled {
            harmonics
                .lmax()
                .checked_add(1)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin-2 scalar conversion factor count",
                })?
        } else {
            0
        };
        let mut spin2_fast_f1 =
            vec![T::from_f64(0.0).expect("zero is representable"); spin2_factor_count];
        let mut spin2_fast_f2 = spin2_fast_f1.clone();
        if spin2_fast_enabled {
            fill_spin2_factors(harmonics.lmax(), &mut spin2_fast_f1, &mut spin2_fast_f2);
        }
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
            spin2_fast_f1,
            spin2_fast_f2,
            legendre_reuse_estimate,
            scalar_legendre_table: OnceLock::new(),
            spin1_scalar_legendre_table: OnceLock::new(),
        })
    }

    pub(crate) fn get_scalar_legendre_table(&self) -> Option<&ScalarLegendreTable<T>> {
        self.scalar_legendre_table
            .get_or_init(|| {
                if self.harmonics.spin() == 0 {
                    ScalarLegendreTable::build_if_beneficial(
                        &self.grid,
                        &self.harmonics,
                        &self.sectoral_norms,
                    )
                } else {
                    None
                }
            })
            .as_ref()
    }

    pub(crate) fn get_safe_scalar_legendre_table_for_synthesis(
        &self,
        coefficients: &[num_complex::Complex<T>],
    ) -> Option<&ScalarLegendreTable<T>> {
        if let Some(table) = self.scalar_legendre_table.get() {
            return table.as_ref();
        }
        if !ScalarLegendreTable::<T>::input_values_are_safe(coefficients) {
            return None;
        }
        self.get_scalar_legendre_table()
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn scalar_legendre_strategy(
        &self,
        operation: LegendreOperation,
    ) -> LegendreStrategy {
        self.scalar_legendre_strategy_with_capacity(operation, current_execution_thread_count())
    }

    pub(crate) fn scalar_legendre_strategy_with_capacity(
        &self,
        operation: LegendreOperation,
        execution_capacity: usize,
    ) -> LegendreStrategy {
        if !ScalarLegendreTable::<T>::supports_grid_and_bandlimit(&self.grid, &self.harmonics) {
            return LegendreStrategy::Recurrence;
        }
        let execution_threads = current_execution_thread_count().min(execution_capacity);
        let strategy = select_scalar_legendre_strategy(
            self.harmonics.lmax(),
            self.harmonics.mmax(),
            self.grid.ntheta(),
            operation,
            execution_threads,
        );
        if strategy == LegendreStrategy::CompactTable
            && !self.legendre_reuse_allows_table_at_thread_count(
                operation,
                self.scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
                execution_threads,
            )
        {
            LegendreStrategy::Recurrence
        } else {
            strategy
        }
    }

    pub(crate) fn legendre_reuse_allows_table(
        &self,
        operation: LegendreOperation,
        table_is_initialized: bool,
    ) -> bool {
        self.legendre_reuse_allows_table_at_thread_count(
            operation,
            table_is_initialized,
            current_execution_thread_count(),
        )
    }

    fn legendre_reuse_allows_table_at_thread_count(
        &self,
        operation: LegendreOperation,
        table_is_initialized: bool,
        execution_threads: usize,
    ) -> bool {
        let Some(expected_calls) = self.legendre_reuse_estimate.calls_for(operation) else {
            return true;
        };
        if expected_calls == 0 {
            return false;
        }
        if table_is_initialized {
            return true;
        }
        measured_table_break_even_calls(
            self.harmonics.lmax(),
            self.harmonics.mmax(),
            self.grid.ntheta(),
            self.grid.nphi(),
            self.harmonics.spin(),
            operation,
            execution_threads,
        )
        .is_some_and(|minimum_calls| expected_calls >= minimum_calls)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn fine_synthesis_ring_group_size(
        &self,
        execution_capacity: usize,
    ) -> Option<usize> {
        let lmax = self.harmonics.lmax();
        if !T::supports_precomputed_scalar()
            || self.grid.kind() != crate::grid::GridKind::GaussLegendre
            || self.grid.ntheta() != lmax + 1
            || self.harmonics.mmax() != lmax
            || !(511..=720).contains(&lmax)
        {
            return None;
        }

        match self.harmonics.spin() {
            0 if execution_capacity >= 4 => Some(32),
            1 if execution_capacity >= 8 && lmax >= 640 => Some(32),
            _ => None,
        }
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
    /// With the `parallel` feature, the initial execution capacity is the
    /// active Rayon pool size, capped at 64. Use
    /// [`Self::prepare_workspace`] to select a different capacity before
    /// execution in another pool.
    ///
    /// # Errors
    ///
    /// Returns an error if any workspace allocation fails.
    pub fn workspace(&self) -> Result<ShtWorkspace<T>, ShtError> {
        let analysis_order_count =
            self.harmonics
                .mmax()
                .checked_add(1)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "analysis order count",
                })?;
        let spin_pair_fast_coefficient_count =
            if self.harmonics.spin() == 1 && T::supports_paired_scalar() {
                self.spin_fast_coefficient_count
            } else {
                0
            };
        let execution_capacity =
            current_execution_thread_count().min(ShtWorkspace::<T>::MAX_EXECUTION_CAPACITY);
        let mut workspace = ShtWorkspace::<T>::new(
            self.spectrum_count,
            self.grid.nphi(),
            self.grid.ntheta(),
            analysis_order_count,
            self.recurrence_factor_count,
            self.spin_recurrence_factor_count,
            self.spin_fast_coefficient_count,
            spin_pair_fast_coefficient_count,
            execution_capacity,
            &self.fft,
            self.cc_resampling.as_ref(),
        )?;
        self.prepare_workspace(&mut workspace, execution_capacity)?;
        Ok(workspace)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn row_fft_parallelism_supported(&self) -> bool {
        self.grid.ntheta() >= 128 && self.grid.nphi() >= 257
    }

    /// Reserves execution scratch and sets the maximum concurrency for a
    /// workspace.
    ///
    /// `execution_capacity` is a workspace budget, independent of the Rayon
    /// pool that runs an operation. A transform uses at most the smaller of
    /// this value and the current pool size. Calling this method with a larger
    /// value opts the workspace into that capacity; a pool change alone does
    /// not increase it. Reducing the value keeps allocated scratch for later
    /// reuse. A capacity of one preserves a serial workspace for batching
    /// independent fields. Parallel builds reserve recurrence scratch for
    /// transform groups and one row-FFT workspace per requested worker.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested capacity is outside the build's
    /// supported range, the workspace is incompatible with this plan, checked
    /// sizing overflows, or scratch allocation fails. On allocation failure,
    /// the prior execution capacity remains active.
    /// In parallel builds, a workspace created for a smaller `mmax` is
    /// incompatible if its analysis order work prefix is too short, including
    /// when the requested execution capacity is one.
    pub fn prepare_workspace(
        &self,
        workspace: &mut ShtWorkspace<T>,
        execution_capacity: usize,
    ) -> Result<(), ShtError> {
        let maximum = ShtWorkspace::<T>::MAX_EXECUTION_CAPACITY;
        if !(1..=maximum).contains(&execution_capacity) {
            return Err(ShtError::InvalidExecutionCapacity {
                requested: execution_capacity,
                maximum,
            });
        }
        self.check_workspace(workspace)?;
        let ring_group_size = ShtWorkspace::<T>::ring_group_size_for_capacity(
            self.grid.ntheta(),
            execution_capacity,
        )?;
        let analysis_order_count =
            self.harmonics
                .mmax()
                .checked_add(1)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "analysis order count",
                })?;
        #[cfg_attr(not(feature = "parallel"), allow(unused_mut))]
        let mut required_worker_count = self
            .grid
            .ntheta()
            .div_ceil(ring_group_size)
            .max(execution_capacity.min(analysis_order_count).max(1));

        #[cfg(feature = "parallel")]
        {
            if let Some(fine_group_size) = self.fine_synthesis_ring_group_size(execution_capacity) {
                required_worker_count = required_worker_count.max(
                    self.grid
                        .ntheta()
                        .div_ceil(fine_group_size.min(self.grid.ntheta())),
                );
            }
            if let Some(hemisphere_workers) = crate::spin_transform::spin1_hemisphere_worker_count(
                self,
                execution_capacity.min(8),
            ) {
                required_worker_count = required_worker_count.max(hemisphere_workers);
            }
            let spin_pair_fast_coefficient_count =
                if self.harmonics.spin() == 1 && T::supports_paired_scalar() {
                    self.spin_fast_coefficient_count
                } else {
                    0
                };
            workspace.ensure_recurrence_worker_count(
                required_worker_count,
                self.recurrence_factor_count,
                self.spin_recurrence_factor_count,
                self.spin_fast_coefficient_count,
                spin_pair_fast_coefficient_count,
                recurrence_scratch_len(workspace.theta_sin.len()),
            )?;
            if self.row_fft_parallelism_supported() {
                let fft_worker_count = execution_capacity.min(self.grid.ntheta());
                if fft_worker_count > 1 {
                    workspace.ensure_fft_worker_count(fft_worker_count, &self.fft)?;
                }
            }
        }

        if workspace.recurrence_workers.len() < required_worker_count {
            return Err(ShtError::BufferLength {
                buffer: "workspace recurrence worker groups",
                expected: required_worker_count,
                actual: workspace.recurrence_workers.len(),
            });
        }

        workspace.ring_group_size = ring_group_size;
        workspace.execution_capacity = execution_capacity;
        Ok(())
    }
}
