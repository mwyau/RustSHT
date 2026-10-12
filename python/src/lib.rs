// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Python extension module for the RustSHT numerical crate.

use pyo3::prelude::*;

#[pymodule]
fn _rustsht(_module: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
