use std::iter::once;

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
    #[allow(dead_code)]
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
            v_subset_instruction: self.v_subset_instruction,
            add_subsets: self.add_subsets,
            mul_subsets: self.mul_subsets,
        }
    }

    #[allow(dead_code)]
    /// Evaluates the layer equation given concrete hints for the subset evaluations
    pub(crate) fn eval<F: Field, E: ExtensionField<F>>(
        &self,
        eval_point: &[Fields<F, E>],
        hints: &[Fields<F, E>],
        b_c_points: &[Fields<F, E>],
    ) -> Fields<F, E> {
        // ensures we have evaluations for all subsets
        // +1 because we need two evaluations for V_{i+1}
        debug_assert_eq!(self.add_subsets.len() + 1, hints.len());

        // determine the number of variables for each subset
        // this determines how we partition the challenge points
        let subset_n_vars = self
            .v_subset_instruction
            .iter()
            .map(|subset| n_vars_from_len(subset.len()))
            .collect::<Vec<_>>();

        // partition challenges
        let (b_points, c_points) = (
            &b_c_points[..subset_n_vars[0]],
            &b_c_points[subset_n_vars[0]..],
        );

        // generate eq tables
        let igz = generate_eq(eval_point);
        let iux = generate_eq(b_points);

        let mut evaluation = Fields::Base(F::zero());

        for (i, hint) in hints.iter().skip(1).enumerate() {
            let c_table = generate_eq(&c_points[..subset_n_vars[i]]);
            let floating_prod: Fields<F, E> =
                c_points[subset_n_vars[i]..].iter().cloned().product();

            // eval current add_i and mul_i
            let add_eval = eval_sparse_entry(&self.add_subsets[i], &igz, &iux, &c_table);
            let mul_eval = eval_sparse_entry(&self.mul_subsets[i], &igz, &iux, &c_table);

            evaluation +=
                floating_prod * (add_eval * (hints[0] + *hint) + mul_eval * hints[0] * *hint);
        }

        evaluation
    }
}

/// Represents components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
#[derive(Debug, Clone)]
pub(crate) struct LayerProvingInfoWithSubset<F: Field, E: ExtensionField<F>> {
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<Fields<F, E>>>,
    /// Instructions on how to extract the v subset values
    /// from an evaluation vector
    pub(crate) v_subset_instruction: Vec<Vec<usize>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

impl<F: Field, E: ExtensionField<F>> LayerProvingInfoWithSubset<F, E> {
    #[allow(dead_code)]
    /// Evaluates all subsets at a given point
    /// subsets only take up to num_var points
    pub(crate) fn eval_subsets(&self, eval_point: &[Fields<F, E>]) -> Vec<Fields<F, E>> {
        // convert subsets to polynomials
        let subset_polys = self
            .v_subsets
            .iter()
            .map(|p| {
                MultilinearPoly::new_extend_to_power_of_two(p.to_vec(), Fields::Base(F::zero()))
            })
            .collect::<Vec<_>>();

        let (b_points, c_points) = (
            &eval_point[..subset_polys[0].num_vars()],
            &eval_point[subset_polys[0].num_vars()..],
        );

        let b_eval = subset_polys[0].evaluate(b_points);

        let c_evals = subset_polys
            .iter()
            .map(|poly| poly.evaluate(&c_points[..poly.num_vars()]));

        once(b_eval).chain(c_evals).collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Subclaim<F: Field, E: ExtensionField<F>> {
    r: Vec<Fields<F, E>>,
    #[allow(dead_code)]
    eval: Fields<F, E>,
    instruction: Vec<(usize, usize)>,
}

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

/// Determine the n_vars given the len of a vector
fn n_vars_from_len(len: usize) -> usize {
    assert_ne!(len, 0);
    if len == 1 {
        1
    } else {
        len.next_power_of_two().ilog2() as usize
    }
}

/// Memory efficient evaluation of a sparse polynomial
/// after all evaluations have been extracted into eq polynomials
fn eval_sparse_entry<F: Field, E: ExtensionField<F>>(
    sparse_entry: &[[usize; 3]],
    igz: &[Fields<F, E>],
    iux: &[Fields<F, E>],
    c_table: &[Fields<F, E>],
) -> Fields<F, E> {
    let mut eval = Fields::Base(F::zero());
    for [z, x, y] in sparse_entry {
        eval += igz[*z] * iux[*x] * c_table[*y];
    }
    eval
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

    use crate::util::{build_agi, n_to_1_folding, n_vars_from_len, Subclaim};

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

    #[test]
    fn test_n_vars_from_len() {
        assert_eq!(n_vars_from_len(1), 1);
        assert_eq!(n_vars_from_len(2), 1);
        assert_eq!(n_vars_from_len(5), 3);
    }
}
