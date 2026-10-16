use rustsht_fft::{Complex, ComplexFftPlan};

fn main() {
    let inverse = std::env::args().nth(1).as_deref() == Some("inverse");
    let n = 2904;
    let mut values: Vec<_> = (0..n)
        .map(|index| {
            let x = (index + 1) as f64;
            Complex::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
        })
        .collect();
    let plan = ComplexFftPlan::<f64>::new(n).expect("N=2904 plan");
    let mut workspace = plan.workspace().expect("N=2904 workspace");
    if inverse {
        plan.inverse(&mut values, &mut workspace)
            .expect("inverse transform");
    } else {
        plan.forward(&mut values, &mut workspace)
            .expect("forward transform");
    }
    for value in values {
        println!("{:.17e} {:.17e}", value.re, value.im);
    }
}
