use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly, vpoly::VPoly};
use sum_check::{SumCheck, interface::SumCheckInterface};
use transcript::Transcript;

use crate::{
    circuit::GeneralCircuit,
    protocol::prover::deposit_subclaims,
    util::{Subclaim, build_agi},
};

use super::VirgoProof;

pub fn verify<F: Field + PrimeField32, E: ExtensionField<F>>(
    circuit: &GeneralCircuit,
    virgo_proof: &VirgoProof<F, E>,
    input: &[Fields<F, E>],
    circuit_output: &[Fields<F, E>],
    transcript: &mut Transcript<F, E>,
) -> Result<bool, &'static str> {
    let output_poly = MultilinearPoly::<F, E>::new_extend_to_power_of_two(
        circuit_output.to_vec(),
        Fields::Base(F::zero()),
    );

    output_poly.commit_to_transcript(transcript);

    let mut r = transcript
        .sample_n_challenges(output_poly.num_vars())
        .into_iter()
        .map(Fields::Extension)
        .collect::<Vec<Fields<F, E>>>();

    let mut claimed_sum = output_poly.evaluate(&r);

    let mut subclaims_container = vec![vec![]; circuit.layers.len()];

    // For each layer
    for i in 0..=circuit.layers.len() - 1 {
        let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[i];

        assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

        let (sumcheck_claimed_sum, b_c_points) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(layer_sumcheck_proof, transcript);

        let layer_proving_info = circuit.generate_layer_proving_info(i);

        let expected_claimed_sum = layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

        // Oracle Check
        assert_eq!(
            sumcheck_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        transcript.observe(layer_sumcheck_hints);

        let subclaims = layer_proving_info.hints_to_subclaims(layer_sumcheck_hints, &b_c_points);

        deposit_subclaims(&mut subclaims_container[i..], subclaims);

        let alphas = transcript
            .sample_n_challenges(subclaims_container[i].len())
            .into_iter()
            .map(Fields::Extension)
            .collect::<Vec<Fields<F, E>>>();

        let folding_info = &virgo_proof.folding_sumchecks[i];

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(&folding_info.0, transcript);

        let table_length = if i == circuit.layers.len() - 1 {
            input.len()
        } else {
            circuit.layers[i + 1].gates.len()
        };

        let agi_x = eval_agi_given_input(
            &alphas,
            &subclaims_container[i],
            table_length,
            &n_to_1_challenges,
        );

        let vi_x = if i == circuit.layers.len() - 1 {
            MultilinearPoly::new_extend_to_power_of_two(
                input.to_vec(),
                Fields::Extension(E::zero()),
            )
            .evaluate(&n_to_1_challenges)
        } else {
            folding_info.1
        };

        // N to 1 Oracle Check
        assert_eq!(n_to_1_claimed_sum, (agi_x * vi_x).to_extension_field());

        transcript.observe(&[vi_x]);

        r = n_to_1_challenges;

        claimed_sum = vi_x;
    }

    Ok(true)
}

pub(crate) fn eval_agi_given_input<F: Field, E: ExtensionField<F>>(
    alphas: &[Fields<F, E>],
    subclaims: &[Subclaim<F, E>],
    table_length: usize,
    challenges: &[Fields<F, E>],
) -> Fields<F, E> {
    let agi = build_agi(alphas, subclaims, table_length);

    let agi_poly = MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

    agi_poly.evaluate(challenges)
}
