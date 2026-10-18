// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustsht::{Complex32, Complex64, ShtPlan, ShtWorkspace};

pub fn widen_real(source: &[f32], target: &mut [f64]) {
    assert_eq!(source.len(), target.len());
    for (target, &source) in target.iter_mut().zip(source) {
        *target = f64::from(source);
    }
}

pub fn narrow_real(source: &[f64], target: &mut [f32]) {
    assert_eq!(source.len(), target.len());
    for (target, &source) in target.iter_mut().zip(source) {
        *target = source as f32;
    }
}

pub fn widen_complex(source: &[Complex32], target: &mut [Complex64]) {
    assert_eq!(source.len(), target.len());
    for (target, source) in target.iter_mut().zip(source) {
        *target = Complex64::new(f64::from(source.re), f64::from(source.im));
    }
}

pub fn narrow_complex(source: &[Complex64], target: &mut [Complex32]) {
    assert_eq!(source.len(), target.len());
    for (target, source) in target.iter_mut().zip(source) {
        *target = Complex32::new(source.re as f32, source.im as f32);
    }
}

/// Reusable f32 input/output bridge over one f64 plan and caller-owned f64 workspace.
///
/// All f64 coefficient and map arrays are allocated once in `new`; each method
/// only widens into those arrays, invokes the f64 operation, and narrows into
/// the caller's f32 output slices.
pub struct ReusableBridge<'a> {
    plan: &'a ShtPlan<f64>,
    workspace: ShtWorkspace<f64>,
    alm0: Vec<Complex64>,
    alm1: Vec<Complex64>,
    map0: Vec<f64>,
    map1: Vec<f64>,
}

impl<'a> ReusableBridge<'a> {
    pub fn new(plan: &'a ShtPlan<f64>, workspace: ShtWorkspace<f64>) -> Self {
        let nalm = plan.harmonics().nalm();
        let nmap = plan.grid().ntheta() * plan.grid().nphi();
        let spin_channels = if plan.harmonics().spin() == 0 { 0 } else { 1 };
        Self {
            plan,
            workspace,
            alm0: vec![Complex64::new(0.0, 0.0); nalm],
            alm1: vec![Complex64::new(0.0, 0.0); spin_channels * nalm],
            map0: vec![0.0; nmap],
            map1: vec![0.0; spin_channels * nmap],
        }
    }

    pub fn buffer_payload_bytes(&self) -> usize {
        self.alm0.len() * std::mem::size_of::<Complex64>()
            + self.alm1.len() * std::mem::size_of::<Complex64>()
            + self.map0.len() * std::mem::size_of::<f64>()
            + self.map1.len() * std::mem::size_of::<f64>()
    }

    pub fn capacities(&self) -> [usize; 4] {
        [
            self.alm0.capacity(),
            self.alm1.capacity(),
            self.map0.capacity(),
            self.map1.capacity(),
        ]
    }

    pub fn synthesis_scalar(&mut self, alm: &[Complex32], map: &mut [f32]) {
        widen_complex(alm, &mut self.alm0);
        self.plan
            .synthesis_into(&self.alm0, &mut self.map0, &mut self.workspace)
            .unwrap();
        narrow_real(&self.map0, map);
    }

    pub fn synthesis_spin1(
        &mut self,
        e_alm: &[Complex32],
        b_alm: &[Complex32],
        first: &mut [f32],
        second: &mut [f32],
    ) {
        widen_complex(e_alm, &mut self.alm0);
        widen_complex(b_alm, &mut self.alm1);
        self.plan
            .synthesis_spin_into(
                &self.alm0,
                &self.alm1,
                &mut self.map0,
                &mut self.map1,
                &mut self.workspace,
            )
            .unwrap();
        narrow_real(&self.map0, first);
        narrow_real(&self.map1, second);
    }

    pub fn analysis_scalar(&mut self, map: &[f32], alm: &mut [Complex32]) {
        widen_real(map, &mut self.map0);
        self.plan
            .analysis_into(&self.map0, &mut self.alm0, &mut self.workspace)
            .unwrap();
        narrow_complex(&self.alm0, alm);
    }

    pub fn analysis_spin1(
        &mut self,
        first: &[f32],
        second: &[f32],
        e_alm: &mut [Complex32],
        b_alm: &mut [Complex32],
    ) {
        widen_real(first, &mut self.map0);
        widen_real(second, &mut self.map1);
        self.plan
            .analysis_spin_into(
                &self.map0,
                &self.map1,
                &mut self.alm0,
                &mut self.alm1,
                &mut self.workspace,
            )
            .unwrap();
        narrow_complex(&self.alm0, e_alm);
        narrow_complex(&self.alm1, b_alm);
    }

    pub fn adjoint_synthesis_scalar(&mut self, map: &[f32], alm: &mut [Complex32]) {
        widen_real(map, &mut self.map0);
        self.plan
            .adjoint_synthesis_into(&self.map0, &mut self.alm0, &mut self.workspace)
            .unwrap();
        narrow_complex(&self.alm0, alm);
    }

    pub fn adjoint_synthesis_spin1(
        &mut self,
        first: &[f32],
        second: &[f32],
        e_alm: &mut [Complex32],
        b_alm: &mut [Complex32],
    ) {
        widen_real(first, &mut self.map0);
        widen_real(second, &mut self.map1);
        self.plan
            .adjoint_synthesis_spin_into(
                &self.map0,
                &self.map1,
                &mut self.alm0,
                &mut self.alm1,
                &mut self.workspace,
            )
            .unwrap();
        narrow_complex(&self.alm0, e_alm);
        narrow_complex(&self.alm1, b_alm);
    }

    pub fn adjoint_analysis_scalar(&mut self, alm: &[Complex32], map: &mut [f32]) {
        widen_complex(alm, &mut self.alm0);
        self.plan
            .adjoint_analysis_into(&self.alm0, &mut self.map0, &mut self.workspace)
            .unwrap();
        narrow_real(&self.map0, map);
    }

    pub fn adjoint_analysis_spin1(
        &mut self,
        e_alm: &[Complex32],
        b_alm: &[Complex32],
        first: &mut [f32],
        second: &mut [f32],
    ) {
        widen_complex(e_alm, &mut self.alm0);
        widen_complex(b_alm, &mut self.alm1);
        self.plan
            .adjoint_analysis_spin_into(
                &self.alm0,
                &self.alm1,
                &mut self.map0,
                &mut self.map1,
                &mut self.workspace,
            )
            .unwrap();
        narrow_real(&self.map0, first);
        narrow_real(&self.map1, second);
    }
}
