use crate::util::{GateAddr, LayerId, LayerProvingInfo};

#[derive(Debug, Clone)]
/// Represents a circuit with gates that can have arbitrary wirings
pub struct GeneralCircuit {
    /// output_layer_index = 0
    pub layers: Vec<Layer>,
}

impl GeneralCircuit {
    pub fn new(layers: Vec<Layer>) -> Self {
        Self { layers }
    }

    /// Determines if circuit is a valid GeneralCircuit
    pub fn verify(&self) -> bool {
        // constraint: all layers must be valid
        self.layers
            .iter()
            .enumerate()
            .all(|(id, layer)| layer.verify(id))
    }

    /// Evaluates the GeneralCircuit given the inputs
    pub fn eval<F>(&self, inputs: &[F]) -> Vec<Vec<F>>
    where
        F: std::ops::Add<F, Output = F>,
        F: std::ops::Mul<F, Output = F>,
        F: std::fmt::Debug + Copy,
    {
        let mut evaluation_scratchpad = vec![vec![]; self.layers.len()];
        evaluation_scratchpad.push(inputs.to_vec());

        for (layer_id, layer) in self.layers.iter().enumerate().rev() {
            evaluation_scratchpad[layer_id] = layer.eval(&evaluation_scratchpad);
        }

        evaluation_scratchpad
    }

    /// Return circuit information needed to run virgo sumcheck
    pub(crate) fn generate_layer_proving_info(&self, layer_id: LayerId) -> LayerProvingInfo {
        // input: constraint: layer_id cannot point to the input layer
        assert_ne!(layer_id, self.layers.len());

        // given some global layer id after the target id
        // converts that to the relative id from the target id
        // example: if target_id = i, then layer i + 1 will have
        // relative id = 0
        let norm_layer_id = |id: LayerId| id - layer_id - 1;

        // determines the number of layers after the target layer
        let rem_layers = self.layers.len() - layer_id;

        // init subset vectors
        let mut v_subset_instruction = vec![vec![]; rem_layers];
        let mut add_subsets = vec![vec![]; rem_layers];
        let mut mul_subsets = vec![vec![]; rem_layers];

        for (gate_index, gate) in self.layers[layer_id].gates.iter().enumerate() {
            // v subset population
            // the goal here is to have a shadow layer for every layer
            // after the target layer.
            // each shadow layer only contains gates that contribute to
            // the input of the target layer.
            // we already initialized empty shadow layers.
            // for each gate input we determine what shadow layer
            // it belongs to and push

            // compute the relative layer index for the inputs
            let [norm_left, norm_right] = [
                norm_layer_id(gate.inputs[0].0),
                norm_layer_id(gate.inputs[1].0),
            ];

            let mut left_sparse_index = 0;
            let mut right_sparse_index = 0;

            if !v_subset_instruction[norm_left].contains(&gate.inputs[0].1) {
                v_subset_instruction[norm_left].push(gate.inputs[0].1);
                left_sparse_index = v_subset_instruction[norm_left].len() - 1;
            }

            if !v_subset_instruction[norm_right].contains(&gate.inputs[1].1) {
                v_subset_instruction[norm_right].push(gate.inputs[1].1);
                right_sparse_index = v_subset_instruction[norm_right].len() - 1;
            }

            // build the add_i / mul_i entry based on v_subset
            let sparse_entry = [gate_index, left_sparse_index, right_sparse_index];

            if gate.op == GateOp::Add {
                add_subsets[norm_left + norm_right].push(sparse_entry);
            } else {
                mul_subsets[norm_left + norm_right].push(sparse_entry);
            }
        }

        LayerProvingInfo {
            layer_id,
            v_subset_instruction,
            add_subsets,
            mul_subsets,
        }
    }
}

#[derive(Debug, Clone)]
/// Represents a Layer in the circuit as a collection of gates
pub struct Layer {
    pub gates: Vec<Gate>,
}

impl Layer {
    pub fn new(gates: Vec<Gate>) -> Self {
        Self { gates }
    }

    /// Detemines if all gates in a given layer have
    /// the appropriate wiring
    pub fn verify(&self, id: LayerId) -> bool {
        // constraint: all gates must be valid
        self.gates.iter().all(|gate| gate.verify(id))
    }

    /// Extracts the gate inputs from the evaluation scratchpad
    /// then applies the gate fn on those inputs
    pub fn eval<F>(&self, evaluation_scratchpad: &[Vec<F>]) -> Vec<F>
    where
        F: std::ops::Add<F, Output = F>,
        F: std::ops::Mul<F, Output = F>,
        F: std::fmt::Debug + Copy,
    {
        self.gates
            .iter()
            .map(|gate| {
                let left_input = &evaluation_scratchpad[gate.inputs[0].0][gate.inputs[0].1];
                let right_input = &evaluation_scratchpad[gate.inputs[1].0][gate.inputs[1].1];
                gate.eval(left_input, right_input)
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Gate Operation enum
pub enum GateOp {
    /// Addition Gate
    Add,
    /// Multiplication Gate
    Mul,
}

#[derive(Debug, Clone)]
/// Represents a node in the circuit tree
pub struct Gate {
    pub op: GateOp,
    pub inputs: [GateAddr; 2],
}

impl Gate {
    pub fn new(op: GateOp, inputs: [GateAddr; 2]) -> Self {
        Self { op, inputs }
    }

    /// Ensures that at least one input gate input comes
    /// from the next layer
    pub fn verify(&self, layer_id: LayerId) -> bool {
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

    /// Applies the gate function to the given inputs
    pub fn eval<F: Copy>(&self, left_input: &F, right_input: &F) -> F
    where
        F: std::ops::Add<F, Output = F>,
        F: std::ops::Mul<F, Output = F>,
    {
        match self.op {
            GateOp::Add => *left_input + *right_input,
            GateOp::Mul => *left_input * *right_input,
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        circuit::{Gate, GateOp, GeneralCircuit, Layer},
        circuit_builder::Builder,
        util::LayerProvingInfo,
    };
    use p3_field::AbstractField;
    use p3_goldilocks::Goldilocks as F;

    // constructs a circuit that peforms a len 3 vector dot product
    // [a, b, c] dot [d, e, f]
    // input layer is given as follows: [a, b, c, d, e, f]
    fn len_three_vector_dot_product_circuit() -> GeneralCircuit {
        GeneralCircuit::new(vec![
            Layer::new(vec![Gate::new(GateOp::Add, [(1, 0), (2, 2)])]),
            Layer::new(vec![Gate::new(GateOp::Add, [(2, 0), (2, 1)])]),
            // element wise multiplication layer
            Layer::new(vec![
                Gate::new(GateOp::Mul, [(3, 0), (3, 3)]),
                Gate::new(GateOp::Mul, [(3, 1), (3, 4)]),
                Gate::new(GateOp::Mul, [(3, 2), (3, 5)]),
            ]),
        ])
    }

    pub fn circuit_1() -> GeneralCircuit {
        let mut builder = Builder::init();

        // input layer
        let a = builder.create_input_node();
        let b = builder.create_input_node();
        let c = builder.create_input_node();
        let d = builder.create_input_node();
        let e = builder.create_input_node();
        let f = builder.create_input_node();

        let g = builder.add_node(a, b, &GateOp::Add);
        let h = builder.add_node(a, b, &GateOp::Mul);
        let j = builder.add_node(c, d, &GateOp::Add);
        let k = builder.add_node(e, f, &GateOp::Add);

        let l = builder.add_node(g, h, &GateOp::Mul);
        let m = builder.add_node(j, d, &GateOp::Add);

        // output layer
        builder.add_node(l, c, &GateOp::Add);
        builder.add_node(m, k, &GateOp::Mul);

        builder.build_circuit()
    }

    #[test]
    fn test_gate_verification() {
        // one input comes from layer 2 and the other from layer 3
        let gate = Gate::new(GateOp::Add, [(2, 0), (3, 0)]);
        // if the gate has to be at layer 1 to be valid
        assert!(gate.verify(1));
        // any other gate value should fail
        assert!(!gate.verify(0));
        assert!(!gate.verify(2));
        assert!(!gate.verify(3));
    }

    #[test]
    fn test_valid_circuit_construction() {
        let circuit = GeneralCircuit::new(vec![
            // output layer
            Layer::new(vec![Gate::new(GateOp::Add, [(1, 0), (2, 0)])]),
            Layer::new(vec![Gate::new(GateOp::Mul, [(3, 0), (2, 0)])]),
            Layer::new(vec![Gate::new(GateOp::Add, [(3, 1), (3, 2)])]),
        ]);
        assert!(circuit.verify())
    }

    #[test]
    fn test_invalid_circuit_construction() {
        let circuit = GeneralCircuit::new(vec![
            // output layer
            Layer::new(vec![Gate::new(GateOp::Add, [(1, 0), (2, 0)])]),
            // problem point, all gates on layer 1 must get at least one
            // input from layer 2
            Layer::new(vec![Gate::new(GateOp::Mul, [(3, 1), (3, 0)])]),
            Layer::new(vec![Gate::new(GateOp::Add, [(3, 1), (3, 2)])]),
        ]);
        assert!(!circuit.verify())
    }

    #[test]
    fn test_gate_eval() {
        let add_gate = Gate::new(GateOp::Add, [(0, 0), (0, 0)]);
        assert_eq!(
            F::from_canonical_u32(32),
            add_gate.eval(&F::from_canonical_u32(12), &F::from_canonical_u32(20))
        );

        let mul_gate = Gate::new(GateOp::Mul, [(0, 0), (0, 0)]);
        assert_eq!(
            F::from_canonical_u32(240),
            mul_gate.eval(&F::from_canonical_u32(12), &F::from_canonical_u32(20))
        );
    }

    #[test]
    fn test_circuit_evaluation() {
        let circuit = len_three_vector_dot_product_circuit();
        let evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .into_iter()
                .map(F::from_canonical_u32)
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            evaluations,
            vec![
                vec![32]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![14]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![4, 10, 18]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![1, 2, 3, 4, 5, 6]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
            ]
        );

        let circuit = circuit_1();
        let evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .into_iter()
                .map(F::from_canonical_u32)
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            evaluations,
            vec![
                vec![9, 121]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![6, 11]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![3, 2, 7, 11]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
                vec![1, 2, 3, 4, 5, 6]
                    .into_iter()
                    .map(F::from_canonical_u32)
                    .collect::<Vec<_>>(),
            ]
        );
    }

    #[test]
    fn test_layer_info_generation() {
        let circuit = circuit_1();
        let output_layer_proving_info = circuit.generate_layer_proving_info(0);
        assert_eq!(
            output_layer_proving_info,
            LayerProvingInfo {
                layer_id: 0,
                v_subset_instruction: vec![vec![0, 1], vec![3], vec![2]],
                add_subsets: vec![vec![], vec![], vec![[0, 0, 0]]],
                mul_subsets: vec![vec![], vec![[1, 1, 0]], vec![]]
            }
        );

        let evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .into_iter()
                .map(F::from_canonical_u32)
                .collect::<Vec<_>>(),
        );

        let layer_info_with_subset = output_layer_proving_info.extract_subsets(&evaluations);
        assert_eq!(
            layer_info_with_subset.v_subsets,
            vec![
                vec![F::from_canonical_u32(6), F::from_canonical_u32(11)],
                vec![F::from_canonical_u32(11)],
                vec![F::from_canonical_u32(3)]
            ]
        );

        let output_layer_proving_info = circuit.generate_layer_proving_info(1);
        assert_eq!(
            output_layer_proving_info,
            LayerProvingInfo {
                layer_id: 1,
                v_subset_instruction: vec![vec![0, 1, 2], vec![3]],
                add_subsets: vec![vec![], vec![[1, 2, 0]]],
                mul_subsets: vec![vec![[0, 0, 1]], vec![]]
            }
        );
    }
}
