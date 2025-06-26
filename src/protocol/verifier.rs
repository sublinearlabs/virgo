use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly, vpoly::VPoly};
use std::marker::PhantomData;
use sum_check::{SumCheck, interface::SumCheckInterface};
use transcript::Transcript;

use crate::{
    circuit::GeneralCircuit,
    util::{Subclaim, build_agi},
};

use super::VirgoProof;

pub struct VirgoVerifier<F, E> {
    _fields: PhantomData<(F, E)>,
}

impl<F: Field + PrimeField32, E: ExtensionField<F>> Default for VirgoVerifier<F, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Field + PrimeField32, E: ExtensionField<F>> VirgoVerifier<F, E> {
    pub fn new() -> Self {
        Self {
            _fields: PhantomData,
        }
    }

    pub fn verify_virgo_proof(
        &self,
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

        // For each layer
        for i in 0..circuit.layers.len() - 1 {
            let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[i];

            assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

            let (sumcheck_claimed_sum, b_c_points) =
                SumCheck::<F, E, VPoly<F, E>>::verify_partial(layer_sumcheck_proof, transcript);

            let layer_proving_info = circuit.generate_layer_proving_info(i);

            let expected_claimed_sum =
                layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

            // Oracle Check
            assert_eq!(
                sumcheck_claimed_sum,
                expected_claimed_sum.to_extension_field()
            );

            transcript.observe(layer_sumcheck_hints);

            let alphas = transcript
                .sample_n_challenges(virgo_proof.layer_subclaims[i].len())
                .into_iter()
                .map(Fields::Extension)
                .collect::<Vec<Fields<F, E>>>();

            let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[i];

            let (n_to_1_claimed_sum, n_to_1_challenges) =
                SumCheck::<F, E, VPoly<F, E>>::verify_partial(n_to_1_sumcheck_proof, transcript);

            let agi_x = eval_agi_given_input(
                &alphas,
                &virgo_proof.layer_subclaims[i],
                circuit.layers[i + 1].gates.len(),
                &n_to_1_challenges,
            );

            transcript.observe(&[*n_to_1_hint]);

            // N to 1 Oracle Check
            assert_eq!(
                n_to_1_claimed_sum,
                (agi_x * *n_to_1_hint).to_extension_field()
            );

            r = n_to_1_challenges;

            claimed_sum = *n_to_1_hint;
        }

        let input_layer_id = circuit.layers.len() - 1;

        let (layer_sumcheck_proof, layer_sumcheck_hints) =
            &virgo_proof.layer_sumchecks[input_layer_id];

        assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

        let (sumcheck_claimed_sum, b_c_points) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(layer_sumcheck_proof, transcript);

        let layer_proving_info = circuit.generate_layer_proving_info(input_layer_id);

        let expected_claimed_sum = layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

        // Oracle Check
        assert_eq!(
            sumcheck_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        transcript.observe(layer_sumcheck_hints);

        let alphas = transcript
            .sample_n_challenges(virgo_proof.layer_subclaims[input_layer_id].len())
            .into_iter()
            .map(Fields::Extension)
            .collect::<Vec<Fields<F, E>>>();

        let (n_to_1_sumcheck_proof, _n_to_1_hint) = &virgo_proof.folding_sumchecks[input_layer_id];

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(n_to_1_sumcheck_proof, transcript);

        let agi_x = eval_agi_given_input(
            &alphas,
            &virgo_proof.layer_subclaims[input_layer_id],
            input.len(),
            &n_to_1_challenges,
        );

        let vi_x = MultilinearPoly::new_extend_to_power_of_two(
            input.to_vec(),
            Fields::Extension(E::zero()),
        )
        .evaluate(&n_to_1_challenges);

        // N to 1 Oracle Check
        assert_eq!(n_to_1_claimed_sum, (agi_x * vi_x).to_extension_field());

        Ok(true)
    }
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
