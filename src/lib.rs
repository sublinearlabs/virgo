/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

/// Represents a circuit with gates that can have arbitrary wirings
pub struct GeneralCircuit {
    layers: Vec<Layer>,
}

/// Represents a Layer in the circuit as a collection of gates
pub struct Layer {
    gates: Vec<Gate>,
}

/// Gate Operation enum
pub enum GateOp {
    /// Addition Gate
    Add,
    /// Multiplication Gate
    Mul,
}

/// Represents a node in the circuit tree
pub struct Gate {
    op: GateOp,
    inputs: [GateAddr; 2],
}
