/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

/// Represents all components needed to perform sumhceck for the `GeneralCircuit`
pub(crate) struct LayerProvingInfo {
    /// Instructions on how to build each v subset
    /// for some given layer id
    v_subsets: Vec<Vec<usize>>,
    /// Subset add i's based on subset v's
    add_subsets: Vec<[usize; 3]>,
    /// Subset mul i's based on subset v's
    mul_subsets: Vec<[usize; 3]>,
}

/// Represents components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
pub(crate) struct LayerProvingInfoWithSubset<F> {
    /// Subset values v for some given layer id
    v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    add_subsets: Vec<[usize; 3]>,
    /// Subset mul i's based on subset v's
    mul_subsets: Vec<[usize; 3]>,
}
