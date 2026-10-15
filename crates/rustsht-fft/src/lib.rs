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
        11, 19, 22, 41, 73, 98, 103, 109, 113, 121, 131, 137, 146, 147, 150, 240, 241, 242, 256,
        257, 300, 363, 484, 512, 513, 525, 683, 722, 723, 726, 1023, 1024, 1025, 1372, 1441, 1442,
        1443, 1452, 2047, 2048, 2049, 2904, 4096, 4097, 12769, 18769,
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

    #[test]
    fn real_radix11_plan_and_execution_routes_cover_terminal_and_interior_layouts() {
        let layouts = [
            (22, 2, 1),
            (121, 1, 11),
            (242, 2, 11),
            (363, 3, 11),
            (726, 6, 11),
            (1023, 3, 31),
            (1441, 1, 131),
        ];

        let single_prime_report = RealFftPlan::<f64>::new(11)
            .expect("standalone radix-11 plan")
            .m0_plan_report();
        assert!(single_prime_report.iter().any(|line| {
            line.contains("real n=11 stage=")
                && line.contains("radix=11")
                && line.contains("route=general prime")
        }));

        for (n, l1, ido) in layouts {
            let plan = RealFftPlan::<f64>::new(n).expect("radix-11 plan");
            let report = plan.m0_plan_report();
            assert!(
                report.iter().any(|line| {
                    line.contains(&format!("radix=11 l1={l1} ido={ido} route=fixed real pass"))
                }),
                "n={n} did not report fixed radix-11 pass: {report:?}"
            );
        }

        for n in [513, 1025, 1443, 1452] {
            let plan = RealFftPlan::<f64>::new(n).expect("control plan");
            let report = plan.m0_plan_report();
            assert!(
                !report.iter().any(|line| {
                    line.contains(&format!("real n={n} stage=")) && line.contains("radix=11")
                }),
                "n={n} unexpectedly uses a native real radix-11 pass: {report:?}"
            );
        }
        assert!(
            RealFftPlan::<f64>::new(1452)
                .unwrap()
                .m0_plan_report()
                .iter()
                .any(|line| line.contains("real n=1452 route=complexify c2c-n=726"))
        );

        let n = 726;
        let plan = RealFftPlan::<f64>::new(n).expect("mixed terminal/interior plan");
        let mut workspace = plan.workspace().expect("radix-11 workspace");
        let input = real_input(n, 1.0);
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        m0_clear_routes();
        plan.forward(&input, &mut spectrum, &mut workspace)
            .expect("radix-11 forward");
        let forward_routes = m0_route_snapshot();
        assert!(forward_routes.iter().any(|route| {
            route.contains("real-forward-pass radix=11") && route.contains("fixed-radix11")
        }));

        m0_clear_routes();
        plan.inverse(&spectrum, &mut vec![0.0; n], &mut workspace)
            .expect("radix-11 inverse");
        let inverse_routes = m0_route_snapshot();
        assert!(inverse_routes.iter().any(|route| {
            route.contains("real-inverse-pass radix=11") && route.contains("fixed-radix11")
        }));
        assert!(
            inverse_routes
                .iter()
                .any(|route| route.contains("real-inverse-terminal:fixed-radix11"))
        );
    }

    #[test]
    fn m3_complex_simd_dispatch_covers_vector_tails_and_paired_groups() {
        use crate::simd::SimdScalar;

        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }

        for (n, radix, ido, vector_ido) in [
            (150, 2, 75, 74),
            (300, 4, 75, 74),
            (726, 2, 363, 362),
            (1452, 4, 363, 362),
            (2904, 8, 363, 362),
            (484, 4, 121, 120),
            (1372, 4, 343, 342),
        ] {
            let plan = ComplexFftPlan::<f64>::new(n).expect("M3 route plan");
            let mut workspace = plan.workspace().expect("M3 route workspace");
            let mut values = complex_input(n);
            m0_clear_routes();
            plan.forward(&mut values, &mut workspace)
                .expect("M3 route forward");
            let event = format!(
                "c2c-avx2:first-prefix n={n} radix={radix} ido={ido} vector_ido={vector_ido} scalar_tail=1"
            );
            assert!(
                m0_route_snapshot().iter().any(|route| route == &event),
                "missing reachable SIMD prefix route for N={n}: {:?}",
                m0_route_snapshot()
            );
        }

        let plan = ComplexFftPlan::<f64>::new(240).expect("nested M3 plan");
        let mut workspace = plan.workspace().expect("nested M3 workspace");
        let mut values = complex_input(240);
        m0_clear_routes();
        plan.forward(&mut values, &mut workspace)
            .expect("nested M3 forward");
        assert!(m0_route_snapshot().iter().any(|route| {
            route == "c2c-avx2:pass-prefix n=240 radix=8 l1=2 ido=15 vector_ido=14 scalar_tail=1"
        }));

        for (n, radix, l1) in [(256, 4, 64), (512, 8, 64), (1024, 8, 128)] {
            let plan = ComplexFftPlan::<f64>::new(n).expect("M3 low-ido plan");
            let mut workspace = plan.workspace().expect("M3 low-ido workspace");
            let mut values = complex_input(n);
            m0_clear_routes();
            plan.forward(&mut values, &mut workspace)
                .expect("M3 low-ido forward");
            let vector_l1 = l1 - l1 % 2;
            let event = format!(
                "c2c-avx2:ido1-pair n={n} radix={radix} l1={l1} vector_l1={vector_l1} scalar_tail=0"
            );
            assert!(
                m0_route_snapshot().iter().any(|route| route == &event),
                "missing ido=1 SIMD route for N={n}: {:?}",
                m0_route_snapshot()
            );
        }

        let plan = ComplexFftPlan::<f64>::new(300).expect("guarded inverse plan");
        let mut workspace = plan.workspace().expect("guarded inverse workspace");
        let mut values = vec![Complex::new(f64::MAX / 16.0, 0.0); 300];
        m0_clear_routes();
        plan.inverse(&mut values, &mut workspace)
            .expect("guarded inverse");
        assert!(
            values
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite())
        );
        let routes = m0_route_snapshot();
        assert!(
            routes
                .iter()
                .any(|route| route.contains("normalization=guarded"))
        );
        assert!(!routes.iter().any(|route| route.contains("c2c-avx2:")));
    }

    #[test]
    fn m3_f32_dispatch_keeps_below_lane_ido_on_scalar_path() {
        let plan = ComplexFftPlan::<f32>::new(12).expect("short f32 plan");
        let mut workspace = plan.workspace().expect("short f32 workspace");
        let mut values = vec![Complex::new(0.25, -0.5); 12];
        m0_clear_routes();
        plan.forward(&mut values, &mut workspace)
            .expect("short f32 forward");

        let routes = m0_route_snapshot();
        assert!(
            routes
                .iter()
                .any(|route| { route == "c2c-scalar:first n=12 radix=4 ido=3" })
        );
        assert!(!routes.iter().any(|route| {
            route == "c2c-avx2:first-prefix n=12 radix=4 ido=3 vector_ido=0 scalar_tail=3"
        }));
    }

    #[test]
    fn m7_real_4097_uses_rader_and_preserves_bluestein_fallback() {
        let n = 4097;
        let plan = RealFftPlan::<f64>::new(n).expect("N=4097 Rader plan");
        let report = plan.m0_plan_report();
        assert!(
            report.iter().any(|line| {
                line.contains("Rader radix=241") && line.contains("convolution=240")
            })
        );
        assert!(
            report
                .iter()
                .any(|line| { line.contains("radix=241 fallback=Bluestein conv=484") })
        );
        assert!(
            report
                .iter()
                .any(|line| { line.contains("radix=241 l1=17 ido=1 route=Rader") })
        );

        let mut workspace = plan.workspace().expect("N=4097 Rader workspace");
        let input = real_input(n, 1.0);
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        m0_clear_routes();
        plan.forward(&input, &mut spectrum, &mut workspace)
            .expect("ordinary Rader forward");
        let ordinary_forward_routes = m0_route_snapshot();
        assert!(
            ordinary_forward_routes
                .iter()
                .any(|route| route == "real-forward:Rader radix=241")
        );
        if <f64 as crate::simd::SimdScalar>::avx2_available() {
            assert!(ordinary_forward_routes.iter().any(|route| {
                route == "c2c-avx2:first-prefix n=240 radix=2 ido=120 vector_ido=120 scalar_tail=0"
            }));
            assert!(ordinary_forward_routes.iter().any(|route| {
                route
                    == "c2c-avx2:pass-prefix n=240 radix=8 l1=2 ido=15 vector_ido=14 scalar_tail=1"
            }));
        }

        m0_clear_routes();
        plan.inverse(&spectrum, &mut vec![0.0; n], &mut workspace)
            .expect("ordinary Rader inverse");
        assert!(
            m0_route_snapshot()
                .iter()
                .any(|route| route == "real-inverse:Rader radix=241")
        );

        let high = f64::MAX / (2.0 * n as f64);
        let high_input = vec![high; n];
        m0_clear_routes();
        plan.forward(&high_input, &mut spectrum, &mut workspace)
            .expect("high-amplitude Bluestein fallback forward");
        let high_forward_routes = m0_route_snapshot();
        assert!(
            high_forward_routes
                .iter()
                .any(|route| route == "real-forward n=4097 route=recursive-safety-fallback")
        );

        let tiny = vec![f64::from_bits(1); n];
        m0_clear_routes();
        plan.forward(&tiny, &mut spectrum, &mut workspace)
            .expect("subnormal Bluestein fallback forward");
        let fallback_routes = m0_route_snapshot();
        assert!(
            fallback_routes
                .iter()
                .any(|route| route == "real-forward n=4097 route=recursive-safety-fallback")
        );
        assert!(
            fallback_routes
                .iter()
                .any(|route| route == "real-forward n=4097 route=recursive")
        );

        let mut high_spectrum = vec![Complex::new(high, -high); plan.spectrum_len()];
        high_spectrum[0].im = 0.0;
        let mut output = vec![0.0; n];
        m0_clear_routes();
        plan.inverse(&high_spectrum, &mut output, &mut workspace)
            .expect("high-amplitude Bluestein fallback inverse");
        assert!(output.iter().all(|value| value.is_finite()));
        let high_inverse_routes = m0_route_snapshot();
        assert!(
            high_inverse_routes
                .iter()
                .any(|route| route == "real-inverse n=4097 route=recursive-safety-fallback")
        );
        assert!(
            high_inverse_routes
                .iter()
                .any(|route| route == "real-inverse n=4097 route=recursive")
        );
    }
}
