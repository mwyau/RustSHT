use rustsht::grid::{GridKind, GridSpec};

fn epsilon(degree: usize, order: usize) -> f64 {
    if degree == order {
        return 0.0;
    }
    let degree = degree as f64;
    let order = order as f64;
    (((degree - order) * (degree + order)) / ((2.0 * degree + 1.0) * (2.0 * degree - 1.0))).sqrt()
}

fn main() {
    let grid = GridSpec::new(GridKind::GaussLegendre, 720, 1440, 0.0).unwrap();
    for (i, &theta) in grid.theta().iter().enumerate() {
        let (sine64, cosine64) = theta.sin_cos();
        let csq64 = if cosine64.abs() > 0.99 {
            (1.0 - sine64) * (1.0 + sine64)
        } else {
            cosine64 * cosine64
        };
        let sine32 = sine64 as f32;
        let cosine32 = cosine64 as f32;
        let csq32 = if cosine32.abs() > 0.99 {
            (1.0_f32 - sine32) * (1.0_f32 + sine32)
        } else {
            cosine32 * cosine32
        };
        println!("G,{i},{theta:.17e},{sine64:.17e},{cosine64:.17e},{csq64:.17e},{sine32:.17e},{cosine32:.17e},{csq32:.17e}");
    }
    for order in 1..=16 {
        let pair_count = (720 - order + 2) / 2;
        let mut alpha64 = 1.0 / epsilon(order + 1, order);
        let alpha1 = epsilon(order + 1, order) / (epsilon(order + 2, order) * epsilon(order + 3, order));
        for pair in 0..pair_count {
            if pair == 1 {
                alpha64 = alpha1;
            } else if pair >= 2 {
                let degree = order + pair * 2;
                let sign = if (pair - 1).is_multiple_of(2) { 1.0 } else { -1.0 };
                alpha64 = sign / (epsilon(degree, order) * epsilon(degree + 1, order) * alpha64);
            }
            let degree = order + pair * 2;
            let sign = if pair.is_multiple_of(2) { 1.0 } else { -1.0 };
            let a = sign * alpha64 * alpha64;
            let b = -a * (epsilon(degree + 2, order).powi(2) + epsilon(degree + 1, order).powi(2));
            let alpha32 = alpha64 as f32;
            let a32 = a as f32;
            let b32 = b as f32;
            let e132 = (epsilon(degree + 1, order) * alpha64) as f32;
            let e232 = (epsilon(degree + 2, order) * alpha64) as f32;
            if (600..=720).contains(&degree) {
                println!("F,{order},{degree},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}", alpha32 as f64, a32 as f64, b32 as f64, e132 as f64, e232 as f64, alpha32 as f64);
            }
        }
    }
}
