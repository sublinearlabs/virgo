pub mod circuit;
mod circuit_builder;
mod circuit_props;

/// Represents all components needed to perform sumhceck for the `GeneralCircuit`
pub(crate) struct LayerProvingInfo {
    // TODO: need to decide if I want to just send the instructions
    //  technically the general circuit should not know anything about F yet
    /// All subsets of v polynomials from the given layer i
    /// to all layers j > i
    //v_subsets: Vec<Vec<F>>,

    /// Subset add i's based on subset v's
    add_subsets: Vec<[usize; 3]>,
    /// Subset mul i's based on subset v's
    mul_subsets: Vec<[usize; 3]>,
}
