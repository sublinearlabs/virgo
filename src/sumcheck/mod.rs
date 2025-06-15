mod phase_one;
mod phase_two;

use p3_field::{ExtensionField, Field, PrimeField32};
use phase_one::prove_phase_one;
use phase_two::prove_phase_two;
use poly::{Fields, utils::generate_eq};
use sum_check::primitives::SumCheckProof;
use transcript::Transcript;

use crate::util::LayerProvingInfoWithSubset;

fn prove_sumcheck_layer<F: Field + PrimeField32, E: ExtensionField<F>>(
    claimed_sum: Fields<F, E>,
    output_point: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<Fields<F, E>>,
    transcript: &mut Transcript<F, E>,
) -> SumCheckProof<F, E> {
    let igz = generate_eq(output_point);

    let phase_one_proof = prove_phase_one(&igz, claimed_sum, layer_proving_info, transcript);

    let phase_two_proof = prove_phase_two(
        &igz,
        &phase_one_proof.challenges,
        layer_proving_info,
        transcript,
    );

    merge_sumcheck_proofs([phase_one_proof, phase_two_proof])
}

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
