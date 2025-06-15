// use libra::utils::{build_phase_one_libra_sumcheck_poly, generate_eq, initialize_phase_one};
use p3_field::{ExtensionField, Field};
use poly::{Fields, utils::generate_eq};

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
        evaluations: &[Vec<F>],
    ) -> LayerProvingInfoWithSubset<F, E> {
        let subset_evaluations = &evaluations[(self.layer_id + 1)..];
        let concrete_subset_values = self
            .v_subset_instruction
            .iter()
            .zip(subset_evaluations)
            .map(|(inst, data)| {
                inst.iter()
                    .map(|index| Fields::Base(data[*index]))
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

type Ahg<F, E> = (Vec<Fields<F, E>>, Vec<Fields<F, E>>, Vec<Fields<F, E>>);

#[allow(dead_code)]
pub fn build_virgo_ahg<F: Field, E: ExtensionField<F>>(
    layer_index: usize,
    circuit_depth: usize,
    igz: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<F, E>,
    total_gates_in_layer: usize,
) -> Ahg<F, E> {
    let depth_from_layer = circuit_depth - layer_index - 1;

    // Get identity polyynomial for each subset
    let mut identity = vec![];

    for layer_index in 0..depth_from_layer {
        identity.push(vec![
            Fields::Extension(E::one());
            layer_proving_info.v_subsets[layer_index].len()
        ]);
    }

    let add_b_ahg = phase_one(
        igz,
        &layer_proving_info.add_subsets,
        &identity,
        depth_from_layer,
        total_gates_in_layer,
    );

    let add_c_ahg = phase_one(
        igz,
        &layer_proving_info.add_subsets,
        &layer_proving_info.v_subsets,
        depth_from_layer,
        total_gates_in_layer,
    );

    let mul_ahg = phase_one(
        igz,
        &layer_proving_info.mul_subsets,
        &layer_proving_info.v_subsets,
        depth_from_layer,
        total_gates_in_layer,
    );

    (add_b_ahg, add_c_ahg, mul_ahg)
}

#[allow(dead_code)]
pub fn phase_one<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    f1: &[Vec<[usize; 3]>],
    vi_subset: &[Vec<Fields<F, E>>],
    depth_from_layer: usize,
    total_gates_in_layer: usize,
) -> Vec<Fields<F, E>> {
    // The total number of inputs to a layer cant be more than (2 * total gates in layer)
    // since the circuit is fan in 2
    let mut res = vec![Fields::Extension(E::zero()); 2 * total_gates_in_layer];

    assert_eq!(f1.len(), depth_from_layer);
    assert_eq!(vi_subset.len(), depth_from_layer);

    for layer_index in 0..f1.len() {
        for [z, x, y] in &f1[layer_index] {
            // It is assumed the operation poly outputs 1 where there is a valid gate
            res[*x] += igz[*z] * vi_subset[layer_index][*y];
        }
    }

    res
}

#[derive(Debug, Clone)]
pub(crate) struct Subclaim<F: Field, E: ExtensionField<F>> {
    r: Vec<Fields<F, E>>,
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

#[cfg(test)]
mod tests {
    use std::vec;

    use p3_field::{AbstractField, extension::BinomialExtensionField};
    use p3_goldilocks::Goldilocks;
    use poly::{
        Fields, MultilinearExtension,
        mle::MultilinearPoly,
        utils::{generate_eq, product_poly},
    };

    type F = Goldilocks;
    type E = BinomialExtensionField<F, 2>;

    use crate::{
        circuit::test::circuit_1,
        util::{Subclaim, build_agi, build_virgo_ahg},
    };

    #[test]
    fn test_build_ahg() {
        // Build circuit
        let circuit = circuit_1();

        // Evaluate circuit on input
        let layer_evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        assert_eq!(
            layer_evaluations[0],
            [9, 121]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>()
        );

        // Generate sumcheck eqn for layer 1
        let layer_index = 1;
        let total_gates_in_layer = 2;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let proving_info_with_subsets = layer_proving_info.extract_subsets(&layer_evaluations);

        let igz = generate_eq::<F, E>(&[Fields::Extension(E::from_canonical_usize(3))]);

        let virgo_ahg = build_virgo_ahg(
            layer_index,
            4,
            &igz,
            &proving_info_with_subsets,
            total_gates_in_layer,
        );

        dbg!(&virgo_ahg);
    }

    #[test]
    fn test_build_agi() {
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
    }
}
