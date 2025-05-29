////! Vigro GKR Circuit properties.
//use crate::circuit::{GateAddr, GateOp, GeneralCircuit};
//
///// This trait defines the properties of a Vigro GKR Circuit.
///// impl on the Circuit, enabling the query of add_mle, mle_mle and v_poly.
//pub trait CircuitProps {
//    /// This the type for the add and mul mle properties of the circuit.
//    type PropsMLE;
//
//    /// Obtain the add and mle mle properties of the circuit given a layer index.
//    fn add_n_mul_mle(&self, layer_index: usize) -> (Self::PropsMLE, Self::PropsMLE);
//}
//
//impl CircuitProps for GeneralCircuit {
//    type PropsMLE = Vec<(usize, GateAddr, GateAddr)>;
//
//    fn add_n_mul_mle(&self, layer_index: usize) -> (Self::PropsMLE, Self::PropsMLE) {
//        assert!(
//            layer_index < self.layers.len(),
//            "Layer index is out of bounds"
//        );
//
//        let mut add_mle = Vec::new();
//        let mut mul_mle = Vec::new();
//
//        for (i, gate) in self.layers[layer_index].gates.iter().enumerate() {
//            match gate.op {
//                GateOp::Add => add_mle.push((i, gate.inputs[0], gate.inputs[1])),
//                GateOp::Mul => mul_mle.push((i, gate.inputs[0], gate.inputs[1])),
//            }
//        }
//
//        (add_mle, mul_mle)
//    }
//}
