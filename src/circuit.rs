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
    pub fn verify(self) -> bool {
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
        assert_ne!(layer_id, self.layers.len() - 1);

        let layer_count = self.layers.len();

        let mut v_subsets = vec![vec![]; layer_count - 1];
        let mut add_subsets = vec![vec![]; layer_count - 1];
        let mut mul_subsets = vec![vec![]; layer_count - 1];

        for (gate_index, gate) in self.layers[layer_id].gates.iter().enumerate() {
            let [(l_layer_id, _), (r_layer_id, _)] = gate.inputs;
            let [norm_l_layer_id, norm_r_layer_id] =
                [layer_count - l_layer_id, layer_count - r_layer_id];

            // populate the v subset vectors
            v_subsets[norm_l_layer_id].push(gate.inputs[0]);
            v_subsets[norm_r_layer_id].push(gate.inputs[1]);

            if gate.op == GateOp::Add {
                add_subsets[norm_l_layer_id + norm_r_layer_id].push([
                    gate_index,
                    v_subsets[norm_l_layer_id].len() - 1,
                    v_subsets[norm_r_layer_id].len() - 1,
                ]);
            } else {
                mul_subsets[norm_l_layer_id + norm_r_layer_id].push([
                    gate_index,
                    v_subsets[norm_l_layer_id].len() - 1,
                    v_subsets[norm_r_layer_id].len() - 1,
                ]);
            }
        }

        LayerProvingInfo {
            v_subsets,
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
mod test {
    use std::thread::Builder;

    use crate::circuit::{Gate, GateOp, GeneralCircuit, Layer};
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

    fn circuit_1() -> GeneralCircuit {
        //let mut builder = Builder::new()
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
    }
}
