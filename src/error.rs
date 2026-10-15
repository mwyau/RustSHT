// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::fmt::{Display, Formatter};

/// Errors returned while constructing a grid or executing an SHT operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShtError {
    /// A grid must contain enough latitude rings for its geometry.
    InvalidThetaCount,
    /// A grid must contain at least one longitude sample per ring.
    InvalidPhiCount,
    /// The longitude origin must be finite.
    NonFinitePhi0,
    /// Allocation of a numerical array failed.
    AllocationFailed { array: &'static str },
    /// Newton iteration did not converge for the indicated positive root.
    QuadratureConvergence { ntheta: usize, root: usize },
    /// Harmonic bounds require `mmax <= lmax`.
    InvalidHarmonicBounds { lmax: usize, mmax: usize },
    /// The spin must not exceed `lmax`.
    SpinExceedsLmax { spin: usize, lmax: usize },
    /// The requested spin operation is not implemented.
    UnsupportedSpin { spin: usize },
    /// Exact analysis has too few latitude rings for the selected geometry.
    AnalysisLatitudeUndersampled { lmax: usize, ntheta: usize },
    /// The requested analysis order exceeds the non-aliased longitude range.
    AnalysisLongitudeUndersampled { mmax: usize, nphi: usize },
    /// The requested synthesis order exceeds the representable longitude range.
    SynthesisLongitudeUndersampled { mmax: usize, nphi: usize },
    /// A packed coefficient array has the wrong number of values.
    CoefficientCount { expected: usize, actual: usize },
    /// A transform input or output buffer has the wrong length.
    BufferLength {
        buffer: &'static str,
        expected: usize,
        actual: usize,
    },
    /// An input coefficient with `m=0` has a nonzero imaginary part.
    NonRealMZero { degree: usize },
    /// An input coefficient below the spin's minimum degree is nonzero.
    NonzeroBelowSpin {
        spin: usize,
        degree: usize,
        order: usize,
    },
    /// The DC or Nyquist value of a real Fourier spectrum is not real.
    NonRealFftEdge { index: usize },
    /// A checked size or offset calculation overflowed `usize`.
    IntegerOverflow { quantity: &'static str },
    /// The requested workspace execution capacity is outside this build's limit.
    InvalidExecutionCapacity { requested: usize, maximum: usize },
}

impl Display for ShtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidThetaCount => {
                f.write_str("ntheta is too small for the selected grid geometry")
            }
            Self::InvalidPhiCount => f.write_str("nphi must be at least 1"),
            Self::NonFinitePhi0 => f.write_str("phi0 must be finite"),
            Self::AllocationFailed { array } => {
                write!(f, "could not allocate the {array} array")
            }
            Self::QuadratureConvergence { ntheta, root } => write!(
                f,
                "Gauss-Legendre Newton iteration failed for ntheta={ntheta}, root={root}"
            ),
            Self::InvalidHarmonicBounds { lmax, mmax } => {
                write!(f, "mmax={mmax} must not exceed lmax={lmax}")
            }
            Self::SpinExceedsLmax { spin, lmax } => {
                write!(f, "spin={spin} must not exceed lmax={lmax}")
            }
            Self::UnsupportedSpin { spin } => {
                write!(f, "spin={spin} operations are not implemented")
            }
            Self::AnalysisLatitudeUndersampled { lmax, ntheta } => write!(
                f,
                "exact analysis through lmax={lmax} requires more latitude rings, received {ntheta}"
            ),
            Self::AnalysisLongitudeUndersampled { mmax, nphi } => write!(
                f,
                "exact analysis through mmax={mmax} requires more than {} longitude samples, received {nphi}",
                mmax.saturating_mul(2)
            ),
            Self::SynthesisLongitudeUndersampled { mmax, nphi } => write!(
                f,
                "synthesis through mmax={mmax} requires at least {} longitude samples, received {nphi}",
                mmax.saturating_mul(2).max(1)
            ),
            Self::CoefficientCount { expected, actual } => {
                write!(
                    f,
                    "expected {expected} packed coefficients, received {actual}"
                )
            }
            Self::BufferLength {
                buffer,
                expected,
                actual,
            } => write!(
                f,
                "expected {expected} values in {buffer}, received {actual}"
            ),
            Self::NonRealMZero { degree } => {
                write!(f, "the m=0 coefficient at degree {degree} must be real")
            }
            Self::NonzeroBelowSpin {
                spin,
                degree,
                order,
            } => write!(
                f,
                "coefficient (l={degree}, m={order}) must be zero for spin={spin}"
            ),
            Self::NonRealFftEdge { index } => {
                write!(
                    f,
                    "real Fourier spectrum edge at index {index} must be real"
                )
            }
            Self::IntegerOverflow { quantity } => {
                write!(f, "{quantity} does not fit in usize")
            }
            Self::InvalidExecutionCapacity { requested, maximum } => write!(
                f,
                "workspace execution capacity must be between 1 and {maximum}, received {requested}"
            ),
        }
    }
}

impl std::error::Error for ShtError {}
