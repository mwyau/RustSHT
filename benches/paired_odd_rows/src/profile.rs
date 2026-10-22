use super::*;

struct PairProfile {
    operation: Operation,
    route: Route,
    plans: Plans,
    samples: Vec<f64>,
    spectra: Vec<C>,
    first: Vec<C>,
    second: Vec<C>,
    source_first: Vec<C>,
    source_second: Vec<C>,
    scratch: Vec<C>,
    output_spectrum: Vec<C>,
    output_real: Vec<f64>,
}

impl PairProfile {
    fn new(
        route: Route,
        operation: Operation,
        n: usize,
        shape: SpectrumShape,
        path: &Path,
    ) -> Self {
        let bins = n / 2 + 1;
        let max_order = match shape {
            SpectrumShape::Full | SpectrumShape::Polar => bins - 1,
            SpectrumShape::Reduced => (bins - 1) / 2,
        };
        let plans = Plans::new(n, max_order + 1);
        let fixture = read_fixture(path, operation, n, 2, bins);
        let (samples, spectra) = match fixture {
            Fixture::Forward { samples } => (samples, Vec::new()),
            Fixture::Inverse { spectra } => (Vec::new(), spectra),
        };
        let scratch_len = plans
            .c2c_forward
            .get_inplace_scratch_len()
            .max(plans.c2c_inverse.get_inplace_scratch_len())
            .max(
                plans
                    .c2c_inverse_mixed
                    .as_ref()
                    .map_or(0, |plan| plan.get_inplace_scratch_len()),
            );
        Self {
            operation,
            route,
            first: vec![C::new(0.0, 0.0); n],
            second: vec![C::new(0.0, 0.0); n],
            source_first: vec![C::new(0.0, 0.0); n],
            source_second: vec![C::new(0.0, 0.0); n],
            scratch: vec![C::new(0.0, 0.0); scratch_len],
            output_spectrum: vec![C::new(0.0, 0.0); plans.modes * 2],
            output_real: vec![0.0; n * 2],
            plans,
            samples,
            spectra,
        }
    }

    fn pack(&mut self) {
        match self.operation {
            Operation::Forward => {
                let (first_samples, second_samples) = self.samples.split_at(self.plans.n);
                if self.route.is_paired() {
                    let component_bound = f64::MAX / (8.0 * self.plans.n as f64);
                    let mut safe = true;
                    for ((slot, &x), &y) in
                        self.first.iter_mut().zip(first_samples).zip(second_samples)
                    {
                        safe &= pair_component_safe(x, component_bound)
                            && pair_component_safe(y, component_bound);
                        *slot = C::new(x, y);
                    }
                    assert!(safe, "profile fixture exceeds paired FFT headroom");
                } else {
                    for ((first, second), (&x, &y)) in self
                        .first
                        .iter_mut()
                        .zip(&mut self.second)
                        .zip(first_samples.iter().zip(second_samples))
                    {
                        *first = C::new(x, 0.0);
                        *second = C::new(y, 0.0);
                    }
                }
            }
            Operation::Inverse => {
                let (first_spectrum, second_spectrum) = self.spectra.split_at(self.plans.bins);
                if self.route.is_paired() {
                    check_real_edges(self.plans.n, first_spectrum);
                    check_real_edges(self.plans.n, second_spectrum);
                    let component_bound = f64::MAX / (8.0 * self.plans.n as f64);
                    let mut safe = complex_components_safe(first_spectrum[0], component_bound)
                        && complex_components_safe(second_spectrum[0], component_bound);
                    self.first.fill(C::new(0.0, 0.0));
                    self.first[0] = C::new(first_spectrum[0].re, second_spectrum[0].re);
                    for k in 1..=self.plans.n / 2 {
                        let x = first_spectrum[k];
                        let y = second_spectrum[k];
                        safe &= complex_components_safe(x, component_bound)
                            && complex_components_safe(y, component_bound);
                        if self.plans.n.is_multiple_of(2) && k == self.plans.n / 2 {
                            self.first[k] = C::new(x.re, y.re);
                        } else {
                            self.first[k] = C::new(x.re - y.im, x.im + y.re);
                            self.first[self.plans.n - k] = C::new(x.re + y.im, -x.im + y.re);
                        }
                    }
                    assert!(safe, "profile fixture exceeds paired FFT headroom");
                } else {
                    pack_hermitian(self.plans.n, first_spectrum, &mut self.first);
                    pack_hermitian(self.plans.n, second_spectrum, &mut self.second);
                }
            }
        }
        black_box(&self.first);
        black_box(&self.second);
    }

    fn save_source(&mut self) {
        self.source_first.copy_from_slice(&self.first);
        self.source_second.copy_from_slice(&self.second);
    }

    fn reload(&mut self) {
        self.first.copy_from_slice(&self.source_first);
        if !self.route.is_paired() {
            self.second.copy_from_slice(&self.source_second);
        }
        black_box(&self.first);
        black_box(&self.second);
    }

    fn transform(&mut self) {
        self.reload();
        match self.operation {
            Operation::Forward => {
                self.plans
                    .c2c_forward
                    .process_with_scratch(&mut self.first, &mut self.scratch);
                if !self.route.is_paired() {
                    self.plans
                        .c2c_forward
                        .process_with_scratch(&mut self.second, &mut self.scratch);
                }
            }
            Operation::Inverse => {
                let plan = match self.route {
                    Route::RustPairMixed | Route::RustReal if self.plans.n == 19 * 19 => self
                        .plans
                        .c2c_inverse_mixed
                        .as_deref()
                        .unwrap_or(self.plans.c2c_inverse.as_ref()),
                    _ => self.plans.c2c_inverse.as_ref(),
                };
                plan.process_with_scratch(&mut self.first, &mut self.scratch);
                if !self.route.is_paired() {
                    plan.process_with_scratch(&mut self.second, &mut self.scratch);
                }
            }
        }
        black_box(&self.first);
        black_box(&self.second);
    }

    fn extract(&mut self) {
        match self.operation {
            Operation::Forward => {
                if self.route.is_paired() {
                    let mut subnormal_output = false;
                    for k in 0..self.plans.modes {
                        let value = self.first[k];
                        let mirror = self.first[(self.plans.n - k) % self.plans.n].conj();
                        let sum = value + mirror;
                        let difference = value - mirror;
                        self.output_spectrum[k] = sum * 0.5;
                        self.output_spectrum[self.plans.modes + k] =
                            C::new(difference.im * 0.5, -difference.re * 0.5);
                        subnormal_output |= complex_has_subnormal(self.output_spectrum[k])
                            || complex_has_subnormal(self.output_spectrum[self.plans.modes + k]);
                    }
                    black_box(subnormal_output);
                } else {
                    self.output_spectrum[..self.plans.modes]
                        .copy_from_slice(&self.first[..self.plans.modes]);
                    self.output_spectrum[self.plans.modes..]
                        .copy_from_slice(&self.second[..self.plans.modes]);
                }
                self.output_spectrum[0].im = 0.0;
                self.output_spectrum[self.plans.modes].im = 0.0;
                if self.plans.n.is_multiple_of(2) && self.plans.modes > self.plans.n / 2 {
                    self.output_spectrum[self.plans.n / 2].im = 0.0;
                    self.output_spectrum[self.plans.modes + self.plans.n / 2].im = 0.0;
                }
                black_box(&self.output_spectrum);
            }
            Operation::Inverse => {
                if self.route.is_paired() {
                    let subnormal_output = self
                        .first
                        .iter()
                        .any(|value| value.re.is_subnormal() || value.im.is_subnormal());
                    black_box(subnormal_output);
                    for j in 0..self.plans.n {
                        self.output_real[j] = self.first[j].re;
                        self.output_real[self.plans.n + j] = self.first[j].im;
                    }
                } else {
                    for j in 0..self.plans.n {
                        self.output_real[j] = self.first[j].re;
                        self.output_real[self.plans.n + j] = self.second[j].re;
                    }
                }
                black_box(&self.output_real);
            }
        }
    }
}

fn pack_hermitian(n: usize, input: &[C], output: &mut [C]) {
    check_real_edges(n, input);
    output.fill(C::new(0.0, 0.0));
    output[0] = C::new(input[0].re, 0.0);
    for k in 1..=n / 2 {
        output[k] = input[k];
        if !(n.is_multiple_of(2) && k == n / 2) {
            output[n - k] = input[k].conj();
        }
    }
}

struct PhaseSummary {
    p10_us: f64,
    median_us: f64,
    p90_us: f64,
}

fn profile_phase<F: FnMut()>(name: &str, mut operation: F) -> PhaseSummary {
    for _ in 0..8 {
        operation();
    }
    let calibration_ms = elapsed_per_call(12, &mut operation);
    let calls = ((TARGET_BATCH.as_secs_f64() * 1.0e3 / calibration_ms).round() as usize).max(1);
    let mut samples = Vec::with_capacity(BATCHES);
    for _ in 0..BATCHES {
        samples.push(elapsed_per_call(calls, &mut operation) * 1000.0);
    }
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let result = PhaseSummary {
        p10_us: percentile(&ordered, 1, 10),
        median_us: ordered[ordered.len() / 2],
        p90_us: percentile(&ordered, 9, 10),
    };
    println!(
        "profile,phase={name},calls={calls},batches={},calibration_us={:.6},p10_us={:.6},median_us={:.6},p90_us={:.6}",
        BATCHES,
        calibration_ms * 1000.0,
        result.p10_us,
        result.median_us,
        result.p90_us,
    );
    for (index, sample) in samples.iter().enumerate() {
        println!("profile_batch,phase={name},index={index},us={sample:.6}");
    }
    result
}

pub(super) fn run_profile(
    route: Route,
    operation: Operation,
    n: usize,
    shape: SpectrumShape,
    path: &Path,
) {
    let mut profile = PairProfile::new(route, operation, n, shape, path);
    println!(
        "profile_environment,route={},operation={},n={},rows=2,shape={},runtime_avx2={},runtime_fma={},mixed_radix_361_inverse={}",
        route.as_str(),
        operation.as_str(),
        n,
        shape.as_str(),
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("fma"),
        operation == Operation::Inverse
            && n == 361
            && matches!(route, Route::RustReal | Route::RustPairMixed),
    );

    let _pack = profile_phase("pack", || profile.pack());
    profile.pack();
    profile.save_source();
    let reload = profile_phase("reload", || profile.reload());
    let fft_plus_reload = profile_phase("fft_plus_reload", || profile.transform());
    println!(
        "profile_estimate,phase=fft_only,median_us={:.6},p10_us={:.6},p90_us={:.6}",
        fft_plus_reload.median_us - reload.median_us,
        fft_plus_reload.p10_us - reload.p10_us,
        fft_plus_reload.p90_us - reload.p90_us,
    );
    profile.pack();
    profile.save_source();
    profile.transform();
    let _extract = profile_phase("extract", || profile.extract());
}
