use std::{
    cmp::max,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::circuit::{Gate, GateOp, GeneralCircuit, Layer};

#[derive(Debug, Clone)]
struct Builder {
    // The number of input
    input_len: usize,
    // The index of the next gate to be added
    current_index: usize,
    // contains a vec of all gates info
    gates: Vec<Arc<Mutex<Node>>>,
}

#[derive(Debug, Clone)]
struct Node {
    // The id of the node in the graph
    id: usize,
    // This is the operation of the gate
    // TODO: make this a reference to an operation
    op: Option<GateOp>,
    // Used for checking if the node has been processed
    is_processed: bool,
    // The left child of the node
    left_child: Option<Arc<Mutex<Node>>>,
    // The right side of the node
    right_child: Option<Arc<Mutex<Node>>>,
    // The layer index of the node after topological sorting
    layer_index: Option<usize>,
    // The index of the gate on its layer after topological sorting
    gate_index: Option<usize>,
}

impl Node {
    pub fn new(
        id: usize,
        op: Option<GateOp>,
        left_child: Option<Arc<Mutex<Node>>>,
        right_child: Option<Arc<Mutex<Node>>>,
    ) -> Node {
        Self {
            id,
            op,
            is_processed: false,
            left_child,
            right_child,
            layer_index: None,
            gate_index: None,
        }
    }

    fn is_input(&self) -> bool {
        self.left_child.is_none() && self.right_child.is_none() && self.op.is_none()
    }

    fn to_gate(&self) -> Gate {
        // Get references to children and acquire locks once
        let left = self.left_child.as_ref().expect("Left child should exist");
        let right = self.right_child.as_ref().expect("Right child should exist");

        // Get left child indices
        let left_guard = left.lock().unwrap();
        let left_layer = left_guard.layer_index.expect("Layer index should be set");
        let left_gate = left_guard.gate_index.expect("Gate index should be set");
        drop(left_guard); // Release left lock before acquiring right lock

        // Get right child indices
        let right_guard = right.lock().unwrap();
        let right_layer = right_guard.layer_index.expect("Layer index should be set");
        let right_gate = right_guard.gate_index.expect("Gate index should be set");
        drop(right_guard);

        // Create gate with collected indices
        let inputs = [(left_layer, left_gate), (right_layer, right_gate)];
        Gate::new(self.op.clone().expect("Operation should exist"), inputs)
    }
}

trait BuilderTrait {
    // Initializes the builder
    fn init(input_len: usize) -> Self;

    // Adds a gate to the circuit and returns the gate index
    fn add_gate(
        &mut self,
        left_child: &Arc<Mutex<Node>>,
        right_child: &Arc<Mutex<Node>>,
        op: &GateOp,
    ) -> Arc<Mutex<Node>>;

    // Creates an input node
    fn create_input_node(&mut self, input_index: usize) -> Arc<Mutex<Node>>;

    // Topologically sorts the circuit to get the various layers
    fn process_circuit(&mut self) -> HashMap<usize, Vec<Arc<Mutex<Node>>>>;

    // Builds the layered circuit
    fn build_circuit(&mut self) -> GeneralCircuit;
}

impl BuilderTrait for Builder {
    // Will the input length always be known before hand?
    fn init(input_len: usize) -> Self {
        Self {
            input_len,
            current_index: input_len - 1,
            gates: vec![],
        }
    }

    fn create_input_node(&mut self, input_index: usize) -> Arc<Mutex<Node>> {
        let node = Arc::new(Mutex::new(Node::new(input_index, None, None, None)));

        self.gates.push(node.clone());

        node
    }

    fn add_gate(
        &mut self,
        left_child: &Arc<Mutex<Node>>,
        right_child: &Arc<Mutex<Node>>,
        op: &GateOp,
    ) -> Arc<Mutex<Node>> {
        self.current_index += 1;

        let node = Arc::new(Mutex::new(Node::new(
            self.current_index,
            Some(op.clone()),
            Some(Arc::clone(left_child)),
            Some(Arc::clone(right_child)),
        )));

        self.gates.push(Arc::clone(&node));

        node
    }

    fn process_circuit(&mut self) -> HashMap<usize, Vec<Arc<Mutex<Node>>>> {
        // Maps layer index to vec of nodes on that layer
        let mut hash_map = HashMap::new();

        // We need to clone the gates vector to avoid ownership issues
        let gates = self.gates.iter().map(Arc::clone).collect::<Vec<_>>();

        for node in gates {
            let mut node_guard = node.lock().unwrap();

            if node_guard.is_input() {
                // Node is an input node
                let layer = hash_map.entry(0).or_insert(Vec::new());
                node_guard.layer_index = Some(0);
                node_guard.gate_index = Some(layer.len());
                node_guard.is_processed = true;
                drop(node_guard); // Release the lock before pushing
                layer.push(Arc::clone(&node));
                continue;
            }

            // Process children if they exist
            if let (Some(left), Some(right)) = (
                node_guard.left_child.as_ref(),
                node_guard.right_child.as_ref(),
            ) {
                let left = Arc::clone(left);
                let right = Arc::clone(right);
                drop(node_guard); // Release lock before processing children

                // Process left child if not already processed
                {
                    let mut left_guard = left.lock().unwrap();
                    if !left_guard.is_processed {
                        drop(left_guard); // Release lock before processing
                        process_node(&left);
                        left_guard = left.lock().unwrap(); // Reacquire lock
                        let layer_idx = left_guard.layer_index.unwrap();
                        let layer = hash_map.entry(layer_idx).or_insert(Vec::new());
                        left_guard.gate_index = Some(layer.len());
                        drop(left_guard); // Release lock before pushing
                        layer.push(Arc::clone(&left));
                    }
                }

                // Process right child if not already processed
                {
                    let mut right_guard = right.lock().unwrap();
                    if !right_guard.is_processed {
                        drop(right_guard); // Release lock before processing
                        process_node(&right);
                        right_guard = right.lock().unwrap(); // Reacquire lock
                        let layer_idx = right_guard.layer_index.unwrap();
                        let layer = hash_map.entry(layer_idx).or_insert(Vec::new());
                        right_guard.gate_index = Some(layer.len());
                        drop(right_guard); // Release lock before pushing
                        layer.push(Arc::clone(&right));
                    }
                }

                process_node(&node);

                // Add node to its layer
                let mut node_guard = node.lock().unwrap();
                let layer_idx = node_guard.layer_index.unwrap();
                let layer = hash_map.entry(layer_idx).or_insert(Vec::new());
                node_guard.gate_index = Some(layer.len());
                drop(node_guard);
                layer.push(Arc::clone(&node));
            }
        }

        hash_map
    }

    fn build_circuit(&mut self) -> GeneralCircuit {
        let map = self.process_circuit();

        let total_layers = map.len();

        dbg!(total_layers);

        // Create a vector filled with empty layers
        let mut all_layers = vec![Layer::new(vec![]); total_layers];

        for (layer_index, layer) in map {
            if layer_index == 0 {
                continue;
            };
            dbg!(&layer_index);
            dbg!(&layer);
            let layer_gates = layer
                .into_iter()
                .map(|node| node.lock().unwrap().to_gate())
                .collect();
            all_layers[layer_index - 1] = Layer::new(layer_gates);
        }

        // all_layers.reverse();

        // all_layers.pop();

        GeneralCircuit::new(all_layers)
    }
}

// Process a node
fn process_node(node: &Arc<Mutex<Node>>) {
    let mut node_guard = node.lock().unwrap();

    // Early return if already processed
    if node_guard.is_processed {
        return;
    }

    let layer_index = if let (Some(left), Some(right)) = (
        node_guard.left_child.as_ref(),
        node_guard.right_child.as_ref(),
    ) {
        let left_layer = left.lock().unwrap().layer_index;
        let right_layer = right.lock().unwrap().layer_index;
        match (left_layer, right_layer) {
            (Some(l), Some(r)) => Some(max(l, r) + 1),
            _ => None,
        }
    } else {
        Some(0) // Input nodes are at layer 0
    };

    node_guard.layer_index = layer_index;
    node_guard.is_processed = true;
}

#[cfg(test)]
mod tests {
    use crate::circuit::GateOp;

    use super::{Builder, BuilderTrait};

    #[test]
    fn test_circuit_builder() {
        let mut builder = Builder::init(4);

        // Build a circuit that does the computation: ax^2 + 3x + 5

        let mul = GateOp::Mul;
        let add = GateOp::Add;

        // Input array = [x,a,3,5]
        // Create input nodes
        let x = builder.create_input_node(0);
        let a = builder.create_input_node(1);
        let three = builder.create_input_node(2);
        let five = builder.create_input_node(3);

        let x_square = builder.add_gate(&x, &x, &mul);

        let a_x_square = builder.add_gate(&a, &x_square, &mul);

        let three_x = builder.add_gate(&three, &x, &mul);

        let a_x_square_plus_three_x = builder.add_gate(&a_x_square, &three_x, &add);

        let res = builder.add_gate(&a_x_square_plus_three_x, &five, &add);

        let circuit = builder.build_circuit();

        dbg!(&circuit);

        // where x = 3 and a = 2
        let ans = circuit.eval(&[3, 2, 3, 5]);

        dbg!(&ans);

        assert_eq!(ans[0][0], 32);
    }
}
