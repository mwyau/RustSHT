// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::fmt::{Display, Formatter};

/// Errors returned while constructing a grid or executing an SHT operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShtError {
    /// A grid must contain at least one latitude ring.
    InvalidThetaCount,
    /// A grid must contain at least one longitude sample per ring.
    InvalidPhiCount,
    /// The longitude origin must be finite.
    NonFinitePhi0,
    /// Allocation of a numerical array failed.
    AllocationFailed { array: &'static str },
    /// Newton iteration did not converge for the indicated positive root.
    QuadratureConvergence { ntheta: usize, root: usize },
}

impl Display for ShtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidThetaCount => f.write_str("ntheta must be at least 1"),
            Self::InvalidPhiCount => f.write_str("nphi must be at least 1"),
            Self::NonFinitePhi0 => f.write_str("phi0 must be finite"),
            Self::AllocationFailed { array } => {
                write!(f, "could not allocate the {array} array")
            }
            Self::QuadratureConvergence { ntheta, root } => write!(
                f,
                "Gauss-Legendre Newton iteration failed for ntheta={ntheta}, root={root}"
            ),
        }
    }
}

impl std::error::Error for ShtError {}
