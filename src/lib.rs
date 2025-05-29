mod circuit;
mod circuit_builder;
mod circuit_props;

/// Represents all components needed to perform sumhceck for the `GeneralCircuit`
struct LayerProvingInfo<F> {
    /// All subsets of v polynomials from the given layer i
    /// to all layers j > i
    v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    add_subsets: Vec<[usize; 3]>,
    /// Subset mul i's based on subset v's
    mul_subsets: Vec<[usize; 3]>,
}
