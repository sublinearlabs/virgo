use std::cmp::max;

use crate::circuit::{Gate, GateOp, GeneralCircuit, Layer};
use crate::util::GateAddr;

#[derive(Debug, Clone)]
struct Builder {
    // contains a vec of all Layers
    layers: Vec<Vec<Node>>,
}

#[derive(Debug, Clone)]
struct Node {
    // This is the operation of the gate
    op: Option<GateOp>,
    // Used for checking if the node has been processed
    is_processed: bool,
    // The left child of the node
    left_child: Option<GateAddr>,
    // The right side of the node
    right_child: Option<GateAddr>,
    // The layer index and gate index of the node
    node_addr: Option<GateAddr>,
}

impl Node {
    pub fn new(
        op: Option<GateOp>,
        left_child: Option<GateAddr>,
        right_child: Option<GateAddr>,
        node_addr: Option<GateAddr>,
    ) -> Node {
        Self {
            op,
            is_processed: false,
            left_child,
            right_child,
            node_addr,
        }
    }

    fn is_input(&self) -> bool {
        self.left_child.is_none() && self.right_child.is_none() && self.op.is_none()
    }

    fn to_gate(&self, max_layer_index: usize) -> Gate {
        // Create gate with left and right children
        let left = (
            max_layer_index - self.left_child.unwrap().0,
            self.left_child.unwrap().1,
        );
        let right = (
            max_layer_index - self.right_child.unwrap().0,
            self.right_child.unwrap().1,
        );
        let inputs = [left, right];
        Gate::new(self.op.clone().expect("Operation should exist"), inputs)
    }
}

impl Builder {
    // Initializes the builder
    fn init() -> Self {
        Self { layers: vec![] }
    }

    // Creates an input node
    fn create_input_node(&mut self) -> GateAddr {
        // Inputs are on layer 0
        if self.layers.len() == 0 {
            self.layers.push(vec![]);
        }

        let input_layer = &mut self.layers[0];

        let gate_index = input_layer.len();

        let node = Node::new(None, None, None, Some((0, gate_index)));

        input_layer.push(node);

        (0, gate_index)
    }

    // Adds a gate to the circuit and returns the gate
    fn add_node(&mut self, left_child: GateAddr, right_child: GateAddr, op: &GateOp) -> GateAddr {
        let node_layer = max(left_child.0, right_child.0) + 1;

        if self.layers.len() <= node_layer {
            self.layers.push(vec![]);
        }

        let layer = &mut self.layers[node_layer];

        let node_index = layer.len();

        let node = Node::new(
            Some(op.clone()),
            Some(left_child),
            Some(right_child),
            Some((node_layer, node_index)),
        );

        layer.push(node);

        (node_layer, node_index)
    }

    fn get_node_mut() {
        todo!()
    }

    // Builds the layered circuit
    fn build_circuit(&mut self) -> GeneralCircuit {
        let max_layer_index = self.layers.len() - 1;

        // Skips the input layer
        let layers = self
            .layers
            .iter()
            .skip(1)
            .map(|layer| {
                Layer::new(
                    layer
                        .iter()
                        .map(|node| node.to_gate(max_layer_index))
                        .collect(),
                )
            })
            .rev()
            .collect();

        GeneralCircuit::new(layers)
    }
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

        let x_square = builder.add_node(x, x, &mul);

        let a_x_square = builder.add_node(a, x_square, &mul);

        let three_x = builder.add_node(three, x, &mul);

        let a_x_square_plus_three_x = builder.add_node(a_x_square, three_x, &add);

        let res = builder.add_node(a_x_square_plus_three_x, five, &add);

        let circuit = builder.build_circuit();

        // where x = 3 and a = 2
        let ans = circuit.eval(&[3, 2, 3, 5]);

        assert_eq!(ans[0][0], 32);
    }
}
