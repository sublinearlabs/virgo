/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

/// Represents a circuit with gates that can have arbitrary wirings
pub struct GeneralCircuit {
    /// output_layer_index = 0
    layers: Vec<Layer>,
}

impl GeneralCircuit {
    fn new(layers: Vec<Layer>) -> Self {
        Self { layers }
    }

    /// Determines if circuit is a valid GeneralCircuit
    fn verify(self) -> bool {
        // constraint: all layers must be valid
        self.layers
            .iter()
            .enumerate()
            .map(|(id, layer)| layer.verify(id))
            .all(|x| x)
    }
}

/// Represents a Layer in the circuit as a collection of gates
pub struct Layer {
    gates: Vec<Gate>,
}

impl Layer {
    fn new(gates: Vec<Gate>) -> Self {
        Self { gates }
    }

    /// Detemines if all gates in a given layer have
    /// the appropriate wiring
    fn verify(&self, id: LayerId) -> bool {
        // constraint: all gates must be valid
        self.gates.iter().map(|gate| gate.verify(id)).all(|x| x)
    }
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

impl Gate {
    fn new(op: GateOp, inputs: [GateAddr; 2]) -> Self {
        Self { op, inputs }
    }

    /// Ensures that at least one input gate input comes
    /// from the next layer
    fn verify(&self, layer_id: LayerId) -> bool {
        let (left_id, right_id) = (self.inputs[0].0, self.inputs[1].0);
        let mut valid = true;

        // constraint 1:
        // all inputs must come from layers j > i
        valid &= left_id > layer_id && right_id > layer_id;

        // constraint 2:
        // at least one gate input must come from layer i + 1
        valid &= left_id == layer_id + 1 || right_id == layer_id + 1;

        valid
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn circuit_construction() {
        todo!()
    }
}
