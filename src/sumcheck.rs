use p3_field::{ExtensionField, Field};
use sum_check::primitives::SumCheckProof;

use crate::util::LayerProvingInfoWithSubset;

// what are  the basic steps I need?
// the goal is to prove the sumcheck relation for a Layer
// the layer proving info with subset should contain all information needed to do this
// hence I should be able to write a function that takes just that and returns a sumcheck proof

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
