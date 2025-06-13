use libra::utils::{build_phase_one_libra_sumcheck_poly, initialize_phase_one};
use p3_field::{ExtensionField, Field};

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
    pub(crate) fn extract_subsets<F: Clone>(
        self,
        evaluations: &[Vec<F>],
    ) -> LayerProvingInfoWithSubset<F> {
        let subset_evaluations = &evaluations[(self.layer_id + 1)..];
        let concrete_subset_values = self
            .v_subset_instruction
            .iter()
            .zip(subset_evaluations)
            .map(|(inst, data)| {
                inst.iter()
                    .map(|index| data[*index].clone())
                    .collect::<Vec<F>>()
            })
            .collect::<Vec<Vec<F>>>();

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
pub(crate) struct LayerProvingInfoWithSubset<F> {
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

pub fn vi_s_n_to_1_folding<F: Field, E: ExtensionField<F>>(
    r_s: &[&[E]],
    vi_evaluations: &[E],
    alphas: &[E],
) {
    let depth_from_layer = vi_evaluations.len();

    assert_eq!(r_s.len(), depth_from_layer);
    assert_eq!(vi_evaluations.len(), depth_from_layer);
    assert_eq!(alphas.len(), depth_from_layer);

    todo!();
}

pub fn build_virgo_ahg<F: Field, E: ExtensionField<F>>(
    layer_index: usize,
    circuit_depth: usize,
    igz: &[E],
    layer_proving_info: &LayerProvingInfoWithSubset<F>,
    total_gates_in_layer: usize,
) -> (Vec<E>, Vec<E>, Vec<E>) {
    let depth_from_layer = circuit_depth - layer_index - 1;

    // TODO: use identity for b
    let add_b_ahg = phase_one(
        igz,
        &layer_proving_info.add_subsets,
        &layer_proving_info.v_subsets,
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

pub fn phase_one<F: Field, E: ExtensionField<F>>(
    igz: &[E],
    f1: &Vec<Vec<[usize; 3]>>,
    vi_subset: &Vec<Vec<F>>,
    depth_from_layer: usize,
    total_gates_in_layer: usize,
) -> Vec<E> {
    // The total number of inputs to a layer cant be more than (2 * total gates in layer)
    // since the circuit is fan in 2
    let mut res = vec![E::zero(); 2 * total_gates_in_layer];

    assert_eq!(f1.len(), depth_from_layer);

    for layer_index in 0..f1.len() {
        for [z, x, y] in &f1[layer_index] {
            // It is assumed the operation poly outputs 1 where there is a valid gate
            res[*x] += igz[*z] * vi_subset[layer_index][*y];
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use libra::utils::generate_eq;
    use p3_field::AbstractField;
    use p3_goldilocks::Goldilocks;

    use crate::{
        circuit::{GateOp, GeneralCircuit, test::circuit_1},
        circuit_builder::Builder,
        util::build_virgo_ahg,
    };

    #[test]
    fn test_n_to_1_folding() {
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

        let layer_evaluation = &layer_evaluations[layer_index];

        let proving_info_with_subsets = layer_proving_info.extract_subsets(&layer_evaluations);

        let igz = generate_eq(
            &[3_usize]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        let virgo_ahg = build_virgo_ahg(
            layer_index,
            4,
            &igz,
            &proving_info_with_subsets,
            total_gates_in_layer,
        );

        dbg!(&virgo_ahg);
    }
}
