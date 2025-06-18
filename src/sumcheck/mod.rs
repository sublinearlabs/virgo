mod phase_one;
mod phase_two;

use p3_field::{ExtensionField, Field, PrimeField32};
use phase_one::prove_phase_one;
use phase_two::prove_phase_two;
use poly::{Fields, utils::generate_eq};
use sum_check::primitives::SumCheckProof;
use transcript::Transcript;

use crate::util::LayerProvingInfoWithSubset;

#[allow(dead_code)]
pub(crate) fn prove_sumcheck_layer<F: Field + PrimeField32, E: ExtensionField<F>>(
    claimed_sum: Fields<F, E>,
    output_point: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<F, E>,
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

#[cfg(test)]
mod test {
    use crate::{circuit::test::circuit_1, sumcheck::prove_sumcheck_layer};
    use p3_field::{AbstractField, ExtensionField, Field, extension::BinomialExtensionField};
    use p3_mersenne_31::Mersenne31 as F;
    use poly::{Fields, MultilinearExtension, mle::MultilinearPoly};
    use sum_check::{SumCheck, interface::SumCheckInterface};
    use transcript::Transcript;
    type E = BinomialExtensionField<F, 3>;

    fn to_fields<F: Field, E: ExtensionField<F>>(values: Vec<u32>) -> Vec<Fields<F, E>> {
        values
            .into_iter()
            .map(|v| Fields::Base(F::from_canonical_u32(v)))
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_prove_and_verify_sumcheck_layer() {
        let circuit = circuit_1();
        let random_value_bank = to_fields(vec![12, 34, 56, 78, 43, 56, 78, 45]);

        let circuit_evals = circuit.eval(&to_fields::<F, E>(vec![1, 2, 3, 4, 5, 6]));

        for i in 0..circuit.layers.len() {
            let layer_mle = MultilinearPoly::new_extend_to_power_of_two(
                circuit_evals[i].clone(),
                Fields::Base(F::zero()),
            );

            let output_point = &random_value_bank[..layer_mle.num_vars()];
            let claimed_sum = layer_mle.evaluate(output_point);

            let layer_proving_info = circuit
                .generate_layer_proving_info(i)
                .extract_subsets(&circuit_evals);

            let mut prover_transcript = Transcript::<F, E>::init();

            let sumcheck_proof = prove_sumcheck_layer(
                claimed_sum,
                output_point,
                &layer_proving_info,
                &mut prover_transcript,
            );

            let mut verifier_transcript = Transcript::<F, E>::init();

            let verification_result = SumCheck::<F, E, MultilinearPoly<F, E>>::verify_partial(
                &sumcheck_proof,
                &mut verifier_transcript,
            );

            assert!(matches!(verification_result, (_, _)));
        }
    }
}
