use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{mle::MultilinearPoly, Fields, MultilinearExtension};
use sum_check::primitives::SumCheckProof;
use transcript::Transcript;

use crate::{
    circuit::GeneralCircuit,
    protocol::sumcheck::prove_sumcheck_layer,
    util::{n_to_1_folding, subclaims_to_hints},
};

use super::VirgoProof;
use crate::util::Subclaim;

/// Prove the correct execution of a `GeneralCircuit`
pub fn prove<F: Field + PrimeField32, E: ExtensionField<F>>(
    circuit: &GeneralCircuit,
    evaluations: &[Vec<Fields<F, E>>],
    transcript: &mut Transcript<F, E>,
) -> VirgoProof<F, E> {
    let mut proof = VirgoProof::<F, E>::default();
    let mut layer_subclaims: Vec<Vec<Subclaim<F, E>>> = vec![vec![]; circuit.layers.len()];

    // commit output to the transcript
    let output_mle =
        MultilinearPoly::new_extend_to_power_of_two(evaluations[0].clone(), Fields::from_u32(0));
    output_mle.commit_to_transcript(transcript);

    // generate layer claim via challenge and evaluation
    let mut eval_point = extension_to_fields(transcript.sample_n_challenges(output_mle.num_vars()));
    let mut eval = output_mle.evaluate(eval_point.as_slice());

    for i in 0..circuit.layers.len() {
        // get info needed to prove the current layer sumcheck relation
        let layer_proving_info = circuit
            .generate_layer_proving_info(i)
            .extract_subsets(evaluations);

        // generate current layer sumcheck proof and
        // generate oracle check hints
        let layer_sumcheck_proof =
            prove_sumcheck_layer(eval, &eval_point, &layer_proving_info, transcript);
        let subclaims = layer_proving_info.eval_subsets(&layer_sumcheck_proof.challenges);
        let hints = subclaims_to_hints(&subclaims);

        // send hints to proof
        // TODO: fix transcript to simplify this step
        transcript.observe_ext_element(
            &hints
                .iter()
                .map(|h| h.to_extension_field())
                .collect::<Vec<_>>(),
        );
        proof.add_layer_proof(layer_sumcheck_proof, hints);

        // distribute the subclaim to their appropriate layers
        deposit_subclaims(&mut layer_subclaims[i..], subclaims);

        // prepare the next layer
        // we do this by folding all subclaims for the next layer into a single claim
        let alphas = extension_to_fields(transcript.sample_n_challenges(layer_subclaims[i].len()));
        let folding_proof: SumCheckProof<F, E> = n_to_1_folding(
            transcript,
            &alphas,
            &layer_subclaims[i],
            &evaluations[i + 1],
        )
        .unwrap();

        // update the evaluation point and the eval
        eval_point = folding_proof.challenges.clone();
        eval = MultilinearPoly::new_extend_to_power_of_two(
            evaluations[i + 1].clone(),
            Fields::from_u32(0),
        )
        .evaluate(&eval_point);

        proof.add_folding_proof(folding_proof, eval);
    }

    proof
}

/// Distributes a set of subclaim belonging to different layers to their
/// appropriate layer entry slot.
fn deposit_subclaims<T>(subclaims_container: &mut [Vec<T>], subclaims: Vec<T>) {
    debug_assert_eq!(subclaims_container.len() + 1, subclaims.len());

    let mut subclaims_iter = subclaims.into_iter();
    subclaims_container[0].push(subclaims_iter.next().unwrap());

    for (entry, subclaim) in subclaims_container.iter_mut().zip(subclaims_iter) {
        entry.push(subclaim);
    }
}

/// Converts a set of extension fields elements to `Fields` type
// TODO: this should be made redaundant once changes are made to `Transcript`
fn extension_to_fields<F: Field, E: ExtensionField<F>>(vals: Vec<E>) -> Vec<Fields<F, E>> {
    vals.into_iter().map(|v| Fields::Extension(v)).collect()
}

#[cfg(test)]
mod test {
    use super::{deposit_subclaims, prove};
    use crate::circuit::test::circuit_1;
    use p3_field::extension::BinomialExtensionField;
    use poly::Fields;

    use p3_mersenne_31::Mersenne31 as F;
    use transcript::Transcript;
    type E = BinomialExtensionField<F, 3>;

    #[test]
    fn test_deposit_subclaims_container() {
        let mut subclaims_container = vec![vec![]; 3];

        deposit_subclaims(&mut subclaims_container, vec![1, 2, 3, 4]);
        assert_eq!(subclaims_container, vec![vec![1, 2], vec![3], vec![4]]);

        deposit_subclaims(&mut subclaims_container[1..], vec![5, 6, 7]);
        assert_eq!(
            subclaims_container,
            vec![vec![1, 2], vec![3, 5, 6], vec![4, 7]]
        );
    }

    #[test]
    fn test_general_circuit_proving() {
        let circuit = circuit_1();
        let evals = circuit.eval(&Fields::<F, E>::from_u32_vec(vec![1, 2, 3, 4, 5, 6]));

        let mut prover_transcript = Transcript::init();
        let _proof = prove(&circuit, &evals, &mut prover_transcript);
    }
}
