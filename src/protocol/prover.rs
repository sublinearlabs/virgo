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

pub fn prove<F: Field + PrimeField32, E: ExtensionField<F>>(
    circuit: &GeneralCircuit,
    evaluations: &[Vec<Fields<F, E>>],
    transcript: &mut Transcript<F, E>,
) -> VirgoProof<F, E> {
    // initialize empty proof
    let mut proof = VirgoProof::<F, E>::default();

    // TODO: this might be just enough for collection of subclaims for the input layer
    //  need to verify this
    let mut layer_subclaims: Vec<Vec<Subclaim<F, E>>> = vec![vec![]; circuit.layers.len()];

    // commit the output mle to the transcript
    let output_mle =
        MultilinearPoly::new_extend_to_power_of_two(evaluations[0].clone(), Fields::from_u32(0));
    output_mle.commit_to_transcript(transcript);

    // sample challenges for the output
    let mut r = extension_to_fields(transcript.sample_n_challenges(output_mle.num_vars()));

    // get layer evaluation
    let mut m = output_mle.evaluate(r.as_slice());

    for i in 0..circuit.layers.len() {
        let layer_proving_info = circuit
            .generate_layer_proving_info(i)
            .extract_subsets(evaluations);

        // TODO: document subsection
        let layer_sumcheck_proof = prove_sumcheck_layer(m, &r, &layer_proving_info, transcript);
        let subclaims = layer_proving_info.eval_subsets(&layer_sumcheck_proof.challenges);
        let hints = subclaims_to_hints(&subclaims);

        // TODO: clean up this observation mechanism
        transcript.observe_ext_element(
            &hints
                .iter()
                .map(|h| h.to_extension_field())
                .collect::<Vec<_>>(),
        );
        proof.add_layer_proof(layer_sumcheck_proof, hints);

        // next we need to deposit subclaims
        deposit_into_subset_info(&mut layer_subclaims, subclaims);

        // sample alphas
        let alphas = extension_to_fields(transcript.sample_n_challenges(layer_subclaims[i].len()));
        let folding_proof: SumCheckProof<F, E> = n_to_1_folding(
            transcript,
            &alphas,
            &layer_subclaims[i],
            &evaluations[i + 1],
        )
        .unwrap();

        // update the evaluation point and the eval
        r = folding_proof.challenges;
        m = MultilinearPoly::new_extend_to_power_of_two(
            evaluations[i + 1].clone(),
            Fields::from_u32(0),
        )
        .evaluate(&r);

        proof.add_folding_proof(folding_proof, m.clone());
    }

    todo!()
}

// TODO: add documentation
fn deposit_into_subset_info<T>(subset_info: &mut [Vec<T>], data: Vec<T>) {
    debug_assert_eq!(subset_info.len() + 1, data.len());

    let mut data_iter = data.into_iter();
    subset_info[0].push(data_iter.next().unwrap());

    for (entry, data) in subset_info.iter_mut().zip(data_iter) {
        entry.push(data);
    }
}

// TODO: make the need for this reduandant
fn extension_to_fields<F: Field, E: ExtensionField<F>>(vals: Vec<E>) -> Vec<Fields<F, E>> {
    vals.into_iter().map(|v| Fields::Extension(v)).collect()
}

#[cfg(test)]
mod test {
    use super::deposit_into_subset_info;

    #[test]
    fn test_deposit_subset_info() {
        let mut subset_info = vec![vec![]; 3];

        deposit_into_subset_info(&mut subset_info, vec![1, 2, 3, 4]);
        assert_eq!(subset_info, vec![vec![1, 2], vec![3], vec![4]]);

        deposit_into_subset_info(&mut subset_info[1..], vec![5, 6, 7]);
        assert_eq!(subset_info, vec![vec![1, 2], vec![3, 5, 6], vec![4, 7]]);
    }
}
