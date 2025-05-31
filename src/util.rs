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
