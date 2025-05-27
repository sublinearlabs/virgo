/// Type alias for layer id
type LayerId = usize;

/// Position of a gate, given it's layer id and index
type GateAddr = (LayerId, usize);

/// Represents a circuit with gates that can have arbitrary wirings
struct GeneralCircuit {
    layers: Vec<Layer>,
}

/// Represents a Layer in the circuit as a collection of gates
struct Layer {
    gates: Vec<Gate>,
}

/// Gate Operation enum
enum GateOp {
    /// Addition Gate
    Add,
    /// Multiplication Gate
    Mul,
}

/// Represents a node in the circuit tree
struct Gate {
    op: GateOp,
    inputs: [GateAddr; 2],
}
