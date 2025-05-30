use std::cmp::max;

use crate::circuit::{Gate, GateAddr, GateOp, GeneralCircuit, Layer};

#[derive(Debug, Clone)]
struct Builder {
    // number_of_input
    input_len: usize,
    // contains a vec of all Layers
    layers: Vec<Vec<Gate>>,
}

impl Builder {
    // Initializes the builder
    fn init() -> Self {
        Self {
            input_len: 0,
            layers: vec![],
        }
    }

    // Creates an input node
    fn create_input_node(&mut self) -> GateAddr {
        let gate_index = self.input_len;

        self.input_len += 1;

        (0, gate_index)
    }

    // Adds a gate to the circuit and returns the gate
    fn add_gate(&mut self, left_child: GateAddr, right_child: GateAddr, op: &GateOp) -> GateAddr {
        let gate_layer = max(left_child.0, right_child.0) + 1;

        if self.layers.len() <= gate_layer - 1 {
            self.layers.push(vec![]);
        }

        let layer = &mut self.layers[gate_layer - 1];

        let gate_index = layer.len();

        let gate = Gate::new(op.clone(), [left_child, right_child]);

        layer.push(gate);

        (gate_layer, gate_index)
    }

    // Builds the layered circuit
    fn build_circuit(&mut self) -> GeneralCircuit {
        let max_layer_index = self.layers.len();

        let layers = self
            .layers
            .clone()
            .into_iter()
            .map(|mut layer| {
                let _ = layer
                    .iter_mut()
                    .map(|gate| update_gate_index(gate, max_layer_index))
                    .collect::<Vec<_>>();
                Layer::new(layer)
            })
            .rev()
            .collect();

        GeneralCircuit::new(layers)
    }
}

pub fn update_gate_index(gate: &mut Gate, max_layer_index: usize) {
    gate.inputs[0].0 = max_layer_index - gate.inputs[0].0;
    gate.inputs[1].0 = max_layer_index - gate.inputs[1].0;
}

#[cfg(test)]
mod tests {
    use crate::circuit::GateOp;

    use super::Builder;

    #[test]
    fn test_circuit_builder() {
        let mut builder = Builder::init();

        // Build a circuit that does the computation: ax^2 + 3x + 5

        let mul = GateOp::Mul;
        let add = GateOp::Add;

        // Input array = [x,a,3,5]
        // Create input nodes
        let x = builder.create_input_node();
        let a = builder.create_input_node();
        let three = builder.create_input_node();
        let five = builder.create_input_node();

        let x_square = builder.add_gate(x, x, &mul);

        let a_x_square = builder.add_gate(a, x_square, &mul);

        let three_x = builder.add_gate(three, x, &mul);

        let a_x_square_plus_three_x = builder.add_gate(a_x_square, three_x, &add);

        let res = builder.add_gate(a_x_square_plus_three_x, five, &add);

        let circuit = builder.build_circuit();

        // where x = 3 and a = 2
        let ans = circuit.eval(&[3, 2, 3, 5]);

        assert_eq!(ans[0][0], 32);
    }
}
