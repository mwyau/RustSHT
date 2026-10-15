// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

//! Standalone one-dimensional FFT crate under development.
//!
//! The crate uses immutable plans with caller-owned execution workspaces.

mod complex;
mod rader;
mod real;
mod real_multipass;
mod simd;

#[cfg(test)]
thread_local! {
    static M0_ROUTE_EVENTS: std::cell::RefCell<Vec<String>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

#[cfg(test)]
pub(crate) fn m0_record_route(event: String) {
    M0_ROUTE_EVENTS.with(|events| {
        let mut events = events.borrow_mut();
        if !events.contains(&event) {
            events.push(event);
        }
    });
}

#[cfg(test)]
pub(crate) fn m0_clear_routes() {
    M0_ROUTE_EVENTS.with(|events| events.borrow_mut().clear());
}

#[cfg(test)]
pub(crate) fn m0_route_snapshot() -> Vec<String> {
    M0_ROUTE_EVENTS.with(|events| events.borrow().clone())
}

pub use complex::{ComplexFftPlan, ComplexFftWorkspace, FftError};
pub use num_complex::{Complex, Complex64};
pub use real::{RealFftPlan, RealFftWorkspace};

#[cfg(test)]
mod m0_diagnostics_tests {
    use super::{Complex, ComplexFftPlan, RealFftPlan, m0_clear_routes, m0_route_snapshot};

    const LENGTHS: &[usize] = &[
        19, 41, 73, 98, 103, 109, 113, 131, 137, 146, 147, 150, 240, 241, 256, 257, 300, 484, 512,
        513, 525, 683, 722, 723, 726, 1023, 1024, 1025, 1372, 1441, 1442, 1443, 1452, 2047, 2048,
        2049, 2904, 4096, 4097, 12769, 18769,
    ];

    fn complex_input(n: usize) -> Vec<Complex<f64>> {
        (0..n)
            .map(|index| {
                let x = (index + 1) as f64;
                Complex::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
            })
            .collect()
    }

    fn real_input(n: usize, scale: f64) -> Vec<f64> {
        (0..n)
            .map(|index| {
                let x = (index + 1) as f64;
                scale * ((0.173 * x).sin() + 0.25 * (0.113 * (x + 1.0)).cos())
            })
            .collect()
    }

    fn spectrum_input(n: usize) -> Vec<Complex<f64>> {
        (0..=n / 2)
            .map(|index| {
                if index == 0 || (n.is_multiple_of(2) && index == n / 2) {
                    Complex::new(0.4 + 0.01 * index as f64, 0.0)
                } else {
                    let x = (index + 1) as f64;
                    Complex::new(0.3 * (0.19 * x).sin(), 0.2 * (0.07 * (x + 1.0)).cos())
                }
            })
            .collect()
    }

    fn print_events(kind: &str, n: usize, direction: &str) {
        for event in m0_route_snapshot() {
            println!("EXEC,{kind},{n},{direction},{event}");
        }
    }

    #[test]
    fn m0_route_visibility_and_memory() {
        println!("RECORD,kind,n,plan_heap_payload_bytes,workspace_bytes,route");
        for &n in LENGTHS {
            let complex_plan = ComplexFftPlan::<f64>::new(n).expect("complex diagnostic plan");
            let complex_workspace = complex_plan
                .workspace()
                .expect("complex diagnostic workspace");
            for line in complex_plan.m0_pass_report() {
                println!("STAGE,c2c,{n},{}", line.replace(',', ";"));
            }
            println!(
                "PLAN,c2c,{n},{},{},-",
                complex_plan.m0_retained_plan_bytes(),
                complex_workspace
                    .scratch_bytes()
                    .expect("complex workspace bytes")
            );
            let mut values = complex_input(n);
            let mut complex_workspace = complex_plan.workspace().expect("complex workspace");
            m0_clear_routes();
            complex_plan
                .forward(&mut values, &mut complex_workspace)
                .expect("complex forward");
            print_events("c2c", n, "forward");
            m0_clear_routes();
            complex_plan
                .inverse(&mut values, &mut complex_workspace)
                .expect("complex inverse");
            print_events("c2c", n, "inverse");

            let real_plan = RealFftPlan::<f64>::new(n).expect("real diagnostic plan");
            let real_workspace = real_plan.workspace().expect("real diagnostic workspace");
            for line in real_plan.m0_plan_report() {
                println!("STAGE,r2c-c2r,{n},{}", line.replace(',', ";"));
            }
            println!(
                "PLAN,real,{n},{},{},-",
                real_plan.m0_retained_plan_bytes(),
                real_workspace
                    .scratch_bytes()
                    .expect("real workspace bytes")
            );
            let mut workspace = real_plan.workspace().expect("real workspace");
            let input = real_input(n, 1.0);
            let mut spectrum = vec![Complex::new(0.0, 0.0); real_plan.spectrum_len()];
            m0_clear_routes();
            real_plan
                .forward(&input, &mut spectrum, &mut workspace)
                .expect("real forward");
            print_events("real", n, "forward");
            let input = spectrum_input(n);
            let mut output = vec![0.0; n];
            m0_clear_routes();
            real_plan
                .inverse(&input, &mut output, &mut workspace)
                .expect("real inverse");
            print_events("real", n, "inverse");
        }

        for n in [257, 723] {
            let plan = RealFftPlan::<f64>::new(n).expect("Rader safety diagnostic plan");
            let mut workspace = plan.workspace().expect("Rader safety workspace");
            let tiny = vec![f64::from_bits(1); n];
            let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
            m0_clear_routes();
            plan.forward(&tiny, &mut spectrum, &mut workspace)
                .expect("tiny real forward");
            print_events("real", n, "forward-subnormal-fallback");
            let tiny = f64::from_bits(1);
            let mut tiny_spectrum = vec![Complex::new(tiny, tiny); plan.spectrum_len()];
            tiny_spectrum[0].im = 0.0;
            if n.is_multiple_of(2) {
                tiny_spectrum[n / 2].im = 0.0;
            }
            let mut output = vec![0.0; n];
            m0_clear_routes();
            plan.inverse(&tiny_spectrum, &mut output, &mut workspace)
                .expect("tiny real inverse");
            print_events("real", n, "inverse-subnormal-fallback");
        }
    }
}
