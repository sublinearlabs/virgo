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
    #[allow(dead_code)]
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

#[allow(dead_code)]
pub fn build_virgo_ahg<F: Field, E: ExtensionField<F>>(
    layer_index: usize,
    circuit_depth: usize,
    igz: &[E],
    layer_proving_info: &LayerProvingInfoWithSubset<F>,
    total_gates_in_layer: usize,
) -> (Vec<E>, Vec<E>, Vec<E>) {
    let depth_from_layer = circuit_depth - layer_index - 1;

    // Get identity polyynomial for each subset
    let mut identity = vec![];

    for layer_index in 0..depth_from_layer {
        identity.push(vec![
            F::one();
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
    igz: &[E],
    f1: &[Vec<[usize; 3]>],
    vi_subset: &[Vec<F>],
    depth_from_layer: usize,
    total_gates_in_layer: usize,
) -> Vec<E> {
    // The total number of inputs to a layer cant be more than (2 * total gates in layer)
    // since the circuit is fan in 2
    let mut res = vec![E::zero(); 2 * total_gates_in_layer];

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

#[allow(dead_code)]
pub(crate) fn build_cki(vi_subset_instruction: &Vec<Vec<usize>>) -> Vec<Vec<(usize, usize)>> {
    let mut res = vec![];

    for subset in vi_subset_instruction {
        let mut layer_res = vec![];
        for (j, _) in subset.iter().enumerate() {
            layer_res.push((j, subset[j]));
        }
        res.push(layer_res);
    }

    res
}

#[cfg(test)]
mod tests {
    use libra::utils::generate_eq;
    use p3_field::AbstractField;
    use p3_goldilocks::Goldilocks;

    use crate::{
        circuit::test::circuit_1,
        util::{build_cki, build_virgo_ahg},
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

    #[test]
    fn test_build_cki() {
        // Build circuit
        let circuit = circuit_1();

        let layer_index = 0;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let cki = &build_cki(&layer_proving_info.v_subset_instruction);

        assert_eq!(cki[0], vec![(0, 0), (1, 1)]);
        assert_eq!(cki[1], vec![(0, 3)]);
        assert_eq!(cki[2], vec![(0, 2)]);
    }
}
