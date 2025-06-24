use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly};
use std::marker::PhantomData;
use sum_check::{SumCheck, interface::SumCheckInterface, sumcheckable::Sumcheckable};
use transcript::Transcript;

use crate::{circuit::GeneralCircuit, util::build_agi};

use super::VirgoProof;

struct VirgoVerifier<F, E, S> {
    _fields: PhantomData<(F, E, S)>,
}

impl<
    F: Field + PrimeField32,
    E: ExtensionField<F>,
    S: SumCheckInterface<F, E> + Clone + Sumcheckable<F, E>,
> VirgoVerifier<F, E, S>
{
    pub fn new() -> Self {
        Self {
            _fields: PhantomData,
        }
    }

    pub fn verify_virgo_proof(
        &self,
        circuit: GeneralCircuit,
        virgo_proof: VirgoProof<F, E>,
        input: Vec<Fields<F, E>>,
        circuit_output: &[Fields<F, E>],
        transcript: &mut Transcript<F, E>,
    ) -> Result<bool, &'static str> {
        let output_poly = MultilinearPoly::<F, E>::new_extend_to_power_of_two(
            circuit_output.to_vec(),
            Fields::Base(F::zero()),
        );

        let mut r = transcript
            .sample_n_challenges(output_poly.num_vars())
            .into_iter()
            .map(|val| Fields::Extension(val))
            .collect::<Vec<Fields<F, E>>>();

        let mut claimed_sum = output_poly.evaluate(&r);

        let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[0];

        assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

        let (sumcheck_claimed_sum, b_c_points) =
            SumCheck::<F, E, S>::verify_partial(&layer_sumcheck_proof, transcript);

        let layer_proving_info = circuit.generate_layer_proving_info(0);

        let expected_claimed_sum = layer_proving_info.eval(&r, &layer_sumcheck_hints, &b_c_points);

        assert_eq!(sumcheck_claimed_sum, expected_claimed_sum);

        let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[0];

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::verify_partial(n_to_1_sumcheck_proof, transcript);

        let mut alphas = transcript
            .sample_n_challenges(layer_sumcheck_hints.len())
            .iter()
            .map(|val| Fields::Extension(*val))
            .collect::<Vec<Fields<F, E>>>();

        let agi = build_agi(&alphas, subclaims, table_length);

        let agi_poly =
            MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

        let g_eval_at_cumcheck_challenges = agi_poly.evaluate(&n_to_1_challenges);

        let res = alphas
            .iter()
            .zip(layer_sumcheck_hints)
            .fold(Fields::Extension(E::zero()), |acc, (lhs, rhs)| {
                acc += lhs * rhs;
                acc
            })
            .collect();

        assert_eq!(res, g_eval_at_cumcheck_challenges * n_to_1_hint);

        claimed_sum = res;
        r = n_to_1_challenges;

        // For each layer
        for i in 1..circuit.layers.len() {
            let (layer_sumcheck_proof, layer_sumcheck_hints) = &virgo_proof.layer_sumchecks[i];

            assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

            let (sumcheck_claimed_sum, b_c_points) =
                SumCheck::<F, E, S>::verify_partial(&layer_sumcheck_proof, transcript);

            let layer_proving_info = circuit.generate_layer_proving_info(i);

            let expected_claimed_sum =
                layer_proving_info.eval(&r, &layer_sumcheck_hints, &b_c_points);

            assert_eq!(sumcheck_claimed_sum, expected_claimed_sum);

            let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[i];

            let (n_to_1_claimed_sum, n_to_1_challenges) =
                SumCheck::verify_partial(n_to_1_sumcheck_proof, transcript);

            let alphas = transcript
                .sample_n_challenges(layer_sumcheck_hints.len())
                .iter()
                .map(|val| Fields::Extension(*val))
                .collect::<Vec<Fields<F, E>>>();

            let agi = build_agi(&alphas, subclaims, table_length);

            let agi_poly =
                MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

            let g_eval_at_cumcheck_challenges = agi_poly.evaluate(&n_to_1_challenges);

            let res = alphas
                .iter()
                .zip(layer_sumcheck_hints)
                .fold(Fields::Extension(E::zero()), |acc, (lhs, rhs)| {
                    acc += lhs * rhs;
                    acc
                })
                .collect();

            assert_eq!(res, g_eval_at_cumcheck_challenges * n_to_1_hint);

            claimed_sum = res;
            r = n_to_1_challenges;
        }

        let input_layer_id = virgo_proof.layer_sumchecks.len() - 1;

        let (layer_sumcheck_proof, layer_sumcheck_hints) =
            &virgo_proof.layer_sumchecks[input_layer_id];

        assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

        let (sumcheck_claimed_sum, b_c_points) =
            SumCheck::<F, E, S>::verify_partial(&layer_sumcheck_proof, transcript);

        let layer_proving_info = circuit.generate_layer_proving_info(input_layer_id);

        let expected_claimed_sum = layer_proving_info.eval(&r, &layer_sumcheck_hints, &b_c_points);

        assert_eq!(sumcheck_claimed_sum, expected_claimed_sum);

        let (n_to_1_sumcheck_proof, n_to_1_hint) = &virgo_proof.folding_sumchecks[input_layer_id];

        let (n_to_1_claimed_sum, n_to_1_challenges) =
            SumCheck::verify_partial(n_to_1_sumcheck_proof, transcript);

        let alphas = transcript
            .sample_n_challenges(layer_sumcheck_hints.len())
            .iter()
            .map(|val| Fields::Extension(*val))
            .collect::<Vec<Fields<F, E>>>();

        let agi = build_agi(&alphas, subclaims, table_length);

        let agi_poly =
            MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

        let g_eval_at_cumcheck_challenges = agi_poly.evaluate(&n_to_1_challenges);

        let res = alphas
            .iter()
            .zip(layer_sumcheck_hints)
            .fold(Fields::Extension(E::zero()), |acc, (lhs, rhs)| {
                acc += lhs * rhs;
                acc
            })
            .collect();

        assert_eq!(res, g_eval_at_cumcheck_challenges * n_to_1_hint);

        claimed_sum = res;

        Ok(true)
    }
}
