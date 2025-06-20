use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly};
use std::marker::PhantomData;
use sum_check::{SumCheck, interface::SumCheckInterface, sumcheckable::Sumcheckable};
use transcript::Transcript;

use crate::circuit::GeneralCircuit;

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

        let mut challenges = transcript
            .sample_n_challenges(output_poly.num_vars())
            .into_iter()
            .map(|val| Fields::Extension(val))
            .collect::<Vec<Fields<F, E>>>();

        let claimed_sum = output_poly.evaluate(&challenges);

        // For each layer
        for i in 0..circuit.layers.len() {
            // Get the layer sumcheck proof
            let (layer_sumcheck_proof, layer_sumcheck_hints) = virgo_proof.layer_sumchecks;

            assert_eq!(claimed_sum, layer_sumcheck_proof.claimed_sum);

            // verify the layer sumcheck proof
            let (claimed_sum, challenges) =
                SumCheck::<F, E, S>::verify_partial(&layer_sumcheck_proof, transcript);

            let (rb, rc) = (
                challenges[..challenges.len() / 2].to_vec(),
                challenges[challenges.len() / 2..].to_vec(),
            );

            let layer_proving_info = circuit.generate_layer_proving_info(i);

            let (n_to_1_proof, n_to_1_hints) = virgo_proof.folding_sumchecks;

            let (n_to_1_claimed_sum, n_to_1_challenges) =
                SumCheck::verify_partial(&n_to_1_proof, transcript);

            let alphas = transcript
                .sample_n_challenges(layer_sumcheck_hints.len())
                .into_iter()
                .map(|val| Fields::Extension(val))
                .collect::<Vec<Fields<F, E>>>();

            let res = eval(
                layer_proving_info,
                alphas,
                layer_sumcheck_hints,
                rb,
                rc,
            );
        }

        Ok(true)
    }
}
