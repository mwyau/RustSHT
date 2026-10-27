use num_complex::Complex64;

use super::*;

#[test]
fn packed_offsets_follow_order_major_layout() {
    let spec = HarmonicSpec::new(3, 2, 0).unwrap();
    assert_eq!(spec.nalm(), 9);
    assert_eq!(spec.index(0, 0), Some(0));
    assert_eq!(spec.index(3, 0), Some(3));
    assert_eq!(spec.index(1, 1), Some(4));
    assert_eq!(spec.index(3, 1), Some(6));
    assert_eq!(spec.index(2, 2), Some(7));
    assert_eq!(spec.index(3, 2), Some(8));
    assert_eq!(spec.index(0, 1), None);
    assert_eq!(spec.index(4, 0), None);
    assert_eq!(spec.index(3, 3), None);
}

#[test]
fn packed_length_matches_triangular_and_rectangular_cases() {
    assert_eq!(HarmonicSpec::scalar(0, 0).unwrap().nalm(), 1);
    assert_eq!(HarmonicSpec::scalar(3, 3).unwrap().nalm(), 10);
    assert_eq!(HarmonicSpec::scalar(3, 1).unwrap().nalm(), 7);
    assert_eq!(HarmonicSpec::scalar(3, 0).unwrap().nalm(), 4);
}

#[test]
fn constructor_rejects_invalid_bounds_and_overflow() {
    assert_eq!(
        HarmonicSpec::new(2, 3, 0),
        Err(ShtError::InvalidHarmonicBounds { lmax: 2, mmax: 3 })
    );
    assert_eq!(
        HarmonicSpec::new(0, 0, 1),
        Err(ShtError::SpinExceedsLmax { spin: 1, lmax: 0 })
    );
    assert_eq!(
        HarmonicSpec::scalar(usize::MAX, 0),
        Err(ShtError::IntegerOverflow {
            quantity: "packed harmonic length"
        })
    );
}

#[test]
fn coefficient_validation_checks_length_reality_and_spin_floor() {
    let scalar = HarmonicSpec::scalar(2, 1).unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); scalar.nalm()];
    alm[scalar.index(2, 1).unwrap()] = Complex64::new(1.0, 2.0);
    assert_eq!(scalar.validate_coefficients(&alm), Ok(()));

    let mut nonreal_m0 = alm.clone();
    nonreal_m0[scalar.index(1, 0).unwrap()].im = 1e-30;
    assert_eq!(
        scalar.validate_coefficients(&nonreal_m0),
        Err(ShtError::NonRealMZero { degree: 1 })
    );
    assert_eq!(
        scalar.validate_coefficients(&alm[..alm.len() - 1]),
        Err(ShtError::CoefficientCount {
            expected: scalar.nalm(),
            actual: scalar.nalm() - 1
        })
    );

    let spin2 = HarmonicSpec::new(3, 2, 2).unwrap();
    let mut spin_alm = vec![Complex64::new(0.0, 0.0); spin2.nalm()];
    spin_alm[spin2.index(2, 0).unwrap()] = Complex64::new(1.0, 0.0);
    assert_eq!(spin2.validate_coefficients(&spin_alm), Ok(()));
    spin_alm[spin2.index(1, 1).unwrap()] = Complex64::new(1.0, 0.0);
    assert_eq!(
        spin2.validate_coefficients(&spin_alm),
        Err(ShtError::NonzeroBelowSpin {
            spin: 2,
            degree: 1,
            order: 1
        })
    );
}
