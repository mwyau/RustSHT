use rustsht_fft::{Complex, ComplexFftPlan};

fn main() {
    let inverse = std::env::args().nth(1).as_deref() == Some("inverse");
    let n = 1442;
    let mut values: Vec<_> = (0..n)
        .map(|index| {
            let x = (index + 1) as f64;
            Complex::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
        })
        .collect();
    let plan = ComplexFftPlan::<f64>::new(n).expect("N=1442 plan");
    let mut workspace = plan.workspace().expect("N=1442 workspace");
    if inverse {
        plan.inverse(&mut values, &mut workspace).expect("inverse");
    } else {
        plan.forward(&mut values, &mut workspace).expect("forward");
    }
    for value in values {
        println!("{:.17e} {:.17e}", value.re, value.im);
    }
}
