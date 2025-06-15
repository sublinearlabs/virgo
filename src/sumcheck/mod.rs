mod phase_one;
mod phase_two;

use p3_field::{ExtensionField, Field, PrimeField32};
use phase_one::prove_phase_one;
use phase_two::prove_phase_two;
use poly::{utils::generate_eq, Fields};
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

#[cfg(test)]
mod test {
    use crate::{circuit::test::circuit_1, sumcheck::prove_sumcheck_layer};
    use p3_field::{extension::BinomialExtensionField, AbstractField, ExtensionField, Field};
    use p3_mersenne_31::Mersenne31 as F;
    use poly::{mle::MultilinearPoly, Fields, MultilinearExtension};
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
        // what is my testing plan
        // 1. need a circuit
        // 2. evaluate that circuit on some given input
        // 3. convert the evaluation entry that I am interested in to a multilinear poly
        // 4. evaluate that at some random point to get a claim
        // 5. generate the layer proving info and transcript
        // 6. pass that to the sumcheck prover
        // 7. partially verify the sumcheck proof (make sure all is well)

        let circuit = circuit_1();

        let circuit_evals = circuit.eval(&to_fields::<F, E>(vec![1, 2, 3, 4, 5, 6]));

        let output_mle = MultilinearPoly::new_extend_to_power_of_two(
            circuit_evals[0].clone(),
            Fields::Base(F::zero()),
        );

        let output_point = to_fields(vec![45]);
        let claimed_sum = output_mle.evaluate(&output_point);

        let output_layer_proving_info = circuit
            .generate_layer_proving_info(0)
            .extract_subsets(&circuit_evals);

        let mut prover_transcript = Transcript::<F, E>::init();

        prove_sumcheck_layer(
            claimed_sum,
            &output_point,
            &output_layer_proving_info,
            &mut prover_transcript,
        );
    }
}
