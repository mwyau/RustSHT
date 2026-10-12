// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

mod scalar;

pub(crate) use scalar::{
    LATITUDE_BLOCK_SIZE, fill_epsilon, for_each_y_lm_dispatched, multiply_complex_scaled,
    polar_order_limit, sectoral_normalizations,
};
