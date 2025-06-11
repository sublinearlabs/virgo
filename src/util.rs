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
pub(crate) struct LayerProvingInfoWithSubset<F> {
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

pub(crate) fn build_cki<F: Field, E: ExtensionField<F>>(
    vi_subset_instruction: &Vec<Vec<usize>>,
    subset: &Vec<Vec<F>>,
) -> Vec<Vec<(usize, usize)>> {
    let mut res = vec![];

    for i in 0..subset.len() {
        let subset = &subset[i];
        let mut layer_res = vec![];
        for j in 0..subset.len() {
            layer_res.push((j, vi_subset_instruction[i][j]));
        }
        res.push(layer_res);
    }

    res
}

#[cfg(test)]
mod tests {
    use p3_field::{AbstractField, extension::BinomialExtensionField};
    use p3_goldilocks::Goldilocks;

    use crate::{circuit::test::circuit_1, util::build_cki};

    #[test]
    fn test_build_cki() {
        // Build circuit
        let circuit = circuit_1();

        // Evaluate circuit on input
        let layer_evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        // Generate sumcheck eqn for layer 0
        let layer_index = 0;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let proving_info_with_subsets = layer_proving_info
            .clone()
            .extract_subsets(&layer_evaluations);

        let cki = &build_cki::<Goldilocks, BinomialExtensionField<Goldilocks, 2>>(
            &layer_proving_info.v_subset_instruction,
            &proving_info_with_subsets.v_subsets,
        );

        assert_eq!(cki[0], vec![(0, 0), (1, 1)]);
        assert_eq!(cki[1], vec![(0, 3)]);
        assert_eq!(cki[2], vec![(0, 2)]);
    }
}
