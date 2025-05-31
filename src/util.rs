/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

#[derive(Debug, PartialEq)]
/// Represents components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
pub(crate) struct LayerProvingInfo {
    /// Layer Id we generated the proving info for
    pub(crate) layer_id: usize,
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<usize>>,
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
            .v_subsets
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

pub(crate) struct LayerProvingInfoWithSubset<F> {
    // TODO: add documentation
    pub(crate) v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}
