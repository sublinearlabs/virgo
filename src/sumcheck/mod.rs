mod phase_one;
mod phase_two;

use p3_field::{ExtensionField, Field};
use sum_check::primitives::SumCheckProof;

/// Utility function to merge two sumcheck proofs
/// used to merge the phase 1 and phase 2 sumcheck proofs
fn merge_sumcheck_proofs<F: Field, E: ExtensionField<F>>(
    proofs: [SumCheckProof<F, E>; 2],
) -> SumCheckProof<F, E> {
    let [proof1, proof2] = proofs;
    SumCheckProof {
        claimed_sum: proof1.claimed_sum,
        round_polynomials: [proof1.round_polynomials, proof2.round_polynomials].concat(),
        challenges: [proof1.challenges, proof2.challenges].concat(),
    }
}
