// use libra::utils::{build_phase_one_libra_sumcheck_poly, generate_eq, initialize_phase_one};
use p3_field::{ExtensionField, Field};
use poly::{
    mle::MultilinearPoly,
    utils::{generate_eq, product_poly},
    vpoly::VPoly,
    Fields, MultilinearExtension,
};
use sum_check::interface::SumCheckInterface;

/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

#[derive(Debug, PartialEq, Clone)]
/// Represents partial components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
pub(crate) struct LayerProvingInfo {
    /// Layer Id we generated the proving info for
    pub(crate) layer_id: usize,
    /// Instructions on how to extract the v subset values
    /// from an evaluation vector
    pub(crate) v_subset_instruction: Vec<Vec<usize>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

impl LayerProvingInfo {
    pub(crate) fn extract_subsets<F: Field, E: ExtensionField<F>>(
        self,
        evaluations: &[Vec<Fields<F, E>>],
    ) -> LayerProvingInfoWithSubset<F, E> {
        let subset_evaluations = &evaluations[(self.layer_id + 1)..];
        let concrete_subset_values = self
            .v_subset_instruction
            .iter()
            .zip(subset_evaluations)
            .map(|(inst, data)| {
                inst.iter()
                    .map(|index| data[*index])
                    .collect::<Vec<Fields<F, E>>>()
            })
            .collect::<Vec<Vec<Fields<F, E>>>>();

        LayerProvingInfoWithSubset {
            v_subsets: concrete_subset_values,
            add_subsets: self.add_subsets,
            mul_subsets: self.mul_subsets,
        }
    }
}

/// Represents components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
#[derive(Debug, Clone)]
pub(crate) struct LayerProvingInfoWithSubset<F: Field, E: ExtensionField<F>> {
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<Fields<F, E>>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

impl<F: Field, E: ExtensionField<F>> LayerProvingInfoWithSubset<F, E> {
    pub(crate) fn eval(&self, hints: &[Fields<F, E>], challenges: &[Fields<F, E>]) -> Fields<F, E> {
        todo!()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Subclaim<F: Field, E: ExtensionField<F>> {
    r: Vec<Fields<F, E>>,
    eval: Fields<F, E>,
    instruction: Vec<(usize, usize)>,
}

#[allow(dead_code)]
pub(crate) fn build_agi<F: Field, E: ExtensionField<F>>(
    alphas: &[Fields<F, E>],
    subclaims: &[Subclaim<F, E>],
    table_length: usize,
) -> Vec<Fields<F, E>> {
    let mut res = vec![Fields::Extension(E::zero()); table_length];

    for k in 0..subclaims.len() {
        let subclaim = &subclaims[k];
        let igz = generate_eq(&subclaim.r);

        for (t, x) in &subclaim.instruction {
            res[*x] += alphas[k] * igz[*t];
        }
    }

    res
}

#[allow(dead_code)]
pub fn n_to_1_folding<F, E, S>(
    transcript: &mut S::Transcript,
    alphas: &[Fields<F, E>],
    subclaims: &[Subclaim<F, E>],
    vi: &[Fields<F, E>],
) -> Result<S::Proof, anyhow::Error>
where
    F: Field,
    E: ExtensionField<F>,
    S: SumCheckInterface<F, E, Polynomial = VPoly<F, E>>,
{
    let agi = build_agi(alphas, subclaims, vi.len());
    let agi_extension =
        MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));
    let vi_poly =
        MultilinearPoly::new_extend_to_power_of_two(vi.to_vec(), Fields::Extension(E::zero()));
    let mut poly = product_poly::<F, E>(vec![vi_poly, agi_extension]);
    let claimed_sum = poly.sum_over_hypercube();
    S::prove_partial(claimed_sum, &mut poly, transcript)
}

/// Returns the index of alement if it exists.
/// If it doesn't pushes and returns the new index
pub(crate) fn push_index<T: PartialEq>(container: &mut Vec<T>, item: T) -> usize {
    if let Some(pos) = container.iter().position(|x| *x == item) {
        pos
    } else {
        container.push(item);
        container.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use p3_field::{extension::BinomialExtensionField, AbstractField};
    use p3_mersenne_31::Mersenne31;
    use poly::{
        mle::MultilinearPoly, utils::product_poly, vpoly::VPoly, Fields, MultilinearExtension,
    };
    use sum_check::{interface::SumCheckInterface, SumCheck};
    use transcript::Transcript;

    type F = Mersenne31;
    type E = BinomialExtensionField<F, 3>;
    type S = SumCheck<F, E, VPoly<F, E>>;

    use crate::util::{build_agi, n_to_1_folding, Subclaim};

    #[test]
    fn test_n_to_1_folding() {
        let main_poly_eval = Fields::from_u32_vec(vec![1, 2, 3, 4, 5, 6]);

        let main_poly = MultilinearPoly::new_extend_to_power_of_two(
            main_poly_eval.clone(),
            Fields::from_u32(0),
        );

        let alphas = Fields::<F, E>::from_u32_vec(vec![2, 3, 5]);

        let all_challenges = Fields::<F, E>::from_u32_vec(vec![3, 4, 5, 2, 3, 4]);

        let c1_subset_poly = MultilinearPoly::new_extend_to_power_of_two(
            Fields::from_u32_vec(vec![1, 3, 5]),
            Fields::from_u32(0),
        );
        let c1_subset_instruction = vec![(0, 0), (1, 2), (2, 4)];
        let c1_r = &all_challenges[..c1_subset_poly.num_vars()];
        let c1_eval = c1_subset_poly.evaluate(c1_r);
        let c1_subclaim = Subclaim {
            r: c1_r.to_vec(),
            eval: c1_eval,
            instruction: c1_subset_instruction,
        };

        let c2_subset_poly = MultilinearPoly::new_extend_to_power_of_two(
            Fields::from_u32_vec(vec![1, 2, 3, 4, 5, 6]),
            Fields::from_u32(0),
        );
        let c2_subset_instruction = vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)];
        let c2_r = &all_challenges[..c2_subset_poly.num_vars()];
        let c2_eval = c2_subset_poly.evaluate(c2_r);
        let c2_subclaim = Subclaim {
            r: c2_r.to_vec(),
            eval: c2_eval,
            instruction: c2_subset_instruction,
        };

        let c3_subset_poly = MultilinearPoly::new_extend_to_power_of_two(
            Fields::from_u32_vec(vec![2, 3, 6]),
            Fields::from_u32(0),
        );
        let c3_subset_instruction = vec![(0, 1), (1, 2), (2, 5)];
        let c3_r = &all_challenges[..c3_subset_poly.num_vars()];
        let c3_eval = c3_subset_poly.evaluate(c3_r);
        let c3_subclaim = Subclaim {
            r: c3_r.to_vec(),
            eval: c3_eval,
            instruction: c3_subset_instruction,
        };

        let subclaims = vec![c1_subclaim, c2_subclaim, c3_subclaim];

        let agi = build_agi(&alphas, &subclaims, main_poly_eval.len());

        let agi_poly =
            MultilinearPoly::new_extend_to_power_of_two(agi, Fields::Extension(E::zero()));

        let res = product_poly(vec![agi_poly, main_poly]).sum_over_hypercube();

        let expected = (alphas[0] * subclaims[0].eval)
            + (alphas[1] * subclaims[1].eval)
            + (alphas[2] * subclaims[2].eval);

        assert_eq!(res, expected);

        let mut prover_transcript = Transcript::<F, E>::init();

        let proof =
            n_to_1_folding::<F, E, S>(&mut prover_transcript, &alphas, &subclaims, &main_poly_eval);

        let mut verifier_transcript = Transcript::<F, E>::init();

        let _verify = S::verify_partial(&proof.unwrap(), &mut verifier_transcript);
    }
}
