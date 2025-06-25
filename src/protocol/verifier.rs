use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly, vpoly::VPoly};
use std::marker::PhantomData;
use sum_check::{SumCheck, interface::SumCheckInterface};
use transcript::Transcript;

use crate::{circuit::GeneralCircuit, util::build_agi};

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
            .map(|val| Fields::Extension(val))
            .collect::<Vec<Fields<F, E>>>();

        let mut claimed_sum = output_poly.evaluate(&r);

        let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[0];

        assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

        let (sumcheck_claimed_sum, b_c_points) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(layer_sumcheck_proof, transcript);

        transcript.observe_ext_element(
            &layer_sumcheck_hints
                .iter()
                .map(|val| val.to_extension_field())
                .collect::<Vec<E>>(),
        );

        let layer_proving_info = circuit.generate_layer_proving_info(0);

        let mut expected_claimed_sum =
            layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

        assert_eq!(
            sumcheck_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        let mut alphas = transcript
            .sample_n_challenges(virgo_proof.layer_subclaims[0].len())
            .iter()
            .map(|val| Fields::Extension(*val))
            .collect::<Vec<Fields<F, E>>>();

        let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[0];

        let agi = build_agi(
            &alphas,
            &virgo_proof.layer_subclaims[0],
            circuit.layers[0].gates.len(),
        );

        let agi_poly =
            MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(n_to_1_sumcheck_proof, transcript);

        let agi_x = agi_poly.evaluate(&n_to_1_challenges);

        transcript.observe_ext_element(&[n_to_1_hint.to_extension_field()]);

        expected_claimed_sum = agi_x * *n_to_1_hint;

        r = n_to_1_challenges;

        assert_eq!(
            n_to_1_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        claimed_sum = *n_to_1_hint;

        // For each layer
        for i in 1..circuit.layers.len() - 1 {
            let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[i];

            assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

            let (sumcheck_claimed_sum, b_c_points) =
                SumCheck::<F, E, VPoly<F, E>>::verify_partial(layer_sumcheck_proof, transcript);

            let layer_proving_info = circuit.generate_layer_proving_info(i);

            expected_claimed_sum = layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

            assert_eq!(
                sumcheck_claimed_sum,
                expected_claimed_sum.to_extension_field()
            );

            transcript.observe_ext_element(
                &layer_sumcheck_hints
                    .iter()
                    .map(|val| val.to_extension_field())
                    .collect::<Vec<E>>(),
            );

            alphas = transcript
                .sample_n_challenges(virgo_proof.layer_subclaims[i].len())
                .iter()
                .map(|val| Fields::Extension(*val))
                .collect::<Vec<Fields<F, E>>>();

            let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[i];

            let agi = build_agi(
                &alphas,
                &virgo_proof.layer_subclaims[i],
                circuit.layers[i + 1].gates.len(),
            );

            let agi_poly =
                MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

            let (n_to_1_claimed_sum, n_to_1_challenges) =
                SumCheck::<F, E, VPoly<F, E>>::verify_partial(n_to_1_sumcheck_proof, transcript);

            let agi_x = agi_poly.evaluate(&n_to_1_challenges);

            transcript.observe_ext_element(&[n_to_1_hint.to_extension_field()]);

            expected_claimed_sum = agi_x * *n_to_1_hint;

            assert_eq!(
                n_to_1_claimed_sum,
                expected_claimed_sum.to_extension_field()
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

        expected_claimed_sum = layer_proving_info.eval(&r, layer_sumcheck_hints, &b_c_points);

        assert_eq!(
            sumcheck_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        transcript.observe_ext_element(
            &layer_sumcheck_hints
                .iter()
                .map(|val| val.to_extension_field())
                .collect::<Vec<E>>(),
        );

        alphas = transcript
            .sample_n_challenges(virgo_proof.layer_subclaims[input_layer_id].len())
            .iter()
            .map(|val| Fields::Extension(*val))
            .collect::<Vec<Fields<F, E>>>();

        let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[input_layer_id];

        let agi = build_agi(
            &alphas,
            &virgo_proof.layer_subclaims[input_layer_id],
            input.len(),
        );

        let agi_poly =
            MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::<F, E, VPoly<F, E>>::verify_partial(n_to_1_sumcheck_proof, transcript);

        let agi_x = agi_poly.evaluate(&n_to_1_challenges);

        let vi_x = MultilinearPoly::new_extend_to_power_of_two(
            input.to_vec(),
            Fields::Extension(E::zero()),
        )
        .evaluate(&n_to_1_challenges);

        transcript.observe_ext_element(&[n_to_1_hint.to_extension_field()]);

        expected_claimed_sum = agi_x * vi_x;

        assert_eq!(
            n_to_1_claimed_sum,
            expected_claimed_sum.to_extension_field()
        );

        Ok(true)
    }
}
