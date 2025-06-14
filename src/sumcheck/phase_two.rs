use p3_field::{ExtensionField, Field, PrimeField32};
use sum_check::primitives::SumCheckProof;

// how do we handle phase 2??

pub(crate) fn prove_phase_two<F: Field + PrimeField32, E: ExtensionField<F>>() -> SumCheckProof<F, E>
{
    todo!()
}
