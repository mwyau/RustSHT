use super::{ANALYSIS_VECTOR_BUFFER_CAPACITY, FixedAnalysisVectorBuffer, RecurrenceWorkspace};
use pulp::{Arch, Simd, WithSimd};

struct VectorSpan<'a>(&'a [f64]);

impl WithSimd for VectorSpan<'_> {
    type Output = (usize, usize, usize);

    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        let (vectors, tail) = S::as_simd_f64s(self.0);
        (S::F64_LANES, vectors.len(), tail.len())
    }
}

#[test]
fn analysis_arrays_are_contiguous_aligned_and_vector_complete() {
    assert!(
        FixedAnalysisVectorBuffer::<f64>::new(
            ANALYSIS_VECTOR_BUFFER_CAPACITY - 1,
            "short fixed analysis buffer"
        )
        .is_err()
    );
    let workspace = RecurrenceWorkspace::<f64>::new(4, 4, 4, 4, ANALYSIS_VECTOR_BUFFER_CAPACITY)
        .expect("fixed-array recurrence workspace");
    let bases = [
        workspace.previous.as_ptr() as usize,
        workspace.current.as_ptr() as usize,
        workspace.next.as_ptr() as usize,
        workspace.analysis_same_real.as_ptr() as usize,
        workspace.analysis_same_imag.as_ptr() as usize,
        workspace.analysis_opposite_real.as_ptr() as usize,
        workspace.analysis_opposite_imag.as_ptr() as usize,
    ];
    let stride = std::mem::size_of::<FixedAnalysisVectorBuffer<f64>>();
    assert_eq!(
        stride,
        ANALYSIS_VECTOR_BUFFER_CAPACITY * std::mem::size_of::<f64>()
    );
    for (index, base) in bases.iter().enumerate() {
        assert_eq!(base % 64, 0, "buffer {index} alignment");
        if index > 0 {
            assert_eq!(*base - bases[index - 1], stride, "buffer {index} stride");
        }
    }
    assert_eq!(std::mem::align_of::<RecurrenceWorkspace<f64>>(), 64);
    assert_eq!(
        std::mem::size_of::<RecurrenceWorkspace<f64>>() % 64,
        0,
        "worker scratch stride avoids sharing its boundary cache line"
    );

    let (width, vector_count, tail_count) =
        Arch::new().dispatch(VectorSpan(&workspace.analysis_same_real));
    assert_eq!(
        vector_count * width + tail_count,
        ANALYSIS_VECTOR_BUFFER_CAPACITY
    );
    assert_eq!(tail_count, 0);
}
