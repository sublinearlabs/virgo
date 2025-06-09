use libra::utils::{build_phase_one_libra_sumcheck_poly, generate_eq, initialize_phase_one};
use p3_field::{AbstractExtensionField, ExtensionField, Field};
use poly::{Fields, MultilinearExtension, mle::MultilinearPoly};

use crate::circuit::{GateOp, GeneralCircuit, Layer};

/// Type alias for layer id
pub type LayerId = usize;

/// Position of a gate, given it's layer id and index
pub type GateAddr = (LayerId, usize);

#[derive(Debug, PartialEq, Clone)]
/// Represents partial components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
pub(crate) struct LayerProvingInfo {
    /// Layer Id we generated the proving info for
    pub(crate) layer_id: usize,
    /// Instructions on how to extract the v subset values
    /// from an evaluation vector
    pub(crate) v_subset_instruction: Vec<Vec<usize>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

impl LayerProvingInfo {
    pub(crate) fn extract_subsets<F: Clone>(
        self,
        evaluations: &[Vec<F>],
    ) -> LayerProvingInfoWithSubset<F> {
        let subset_evaluations = &evaluations[(self.layer_id + 1)..];
        let concrete_subset_values = self
            .v_subset_instruction
            .iter()
            .zip(subset_evaluations)
            .map(|(inst, data)| {
                inst.iter()
                    .map(|index| data[*index].clone())
                    .collect::<Vec<F>>()
            })
            .collect::<Vec<Vec<F>>>();

        LayerProvingInfoWithSubset {
            v_subsets: concrete_subset_values,
            add_subsets: self.add_subsets,
            mul_subsets: self.mul_subsets,
        }
    }
}

/// Represents components needed to perform sumcheck for the `GeneralCircuit`
/// with concrete subset values
#[derive(Debug, Clone)]
pub(crate) struct LayerProvingInfoWithSubset<F> {
    /// Subset values v for some given layer id
    pub(crate) v_subsets: Vec<Vec<F>>,
    /// Subset add i's based on subset v's
    pub(crate) add_subsets: Vec<Vec<[usize; 3]>>,
    /// Subset mul i's based on subset v's
    pub(crate) mul_subsets: Vec<Vec<[usize; 3]>>,
}

pub fn vi_s_n_to_1_folding<F: Field, E: ExtensionField<F>>(
    r_s: &[&[E]],
    vi_evaluations: &[E],
    cki: Vec<E>,
    alphas: &[E],
) {
    todo!();
}

fn build_cki<F: Field, E: ExtensionField<F>>(
    circuit: GeneralCircuit,
    layer_id: usize,
    evaluations: Vec<Vec<F>>,
) {
    let circuit_info = generate_circuit_info_for_cki(&circuit, layer_id).1;
    let layer_proving_info = circuit.generate_layer_proving_info(layer_id);
    let layer_proving_info_with_subset = layer_proving_info
        .clone()
        .extract_subsets(&evaluations)
        .v_subsets;
    let layer_proving_info = circuit.generate_layer_proving_info(layer_id).mul_subsets;

    dbg!(&layer_proving_info_with_subset);

    for layer_index in 0..layer_proving_info.len() {
        let layer = &layer_proving_info[layer_index];
        for [z, x, y] in layer {
            dbg!("Subset", [z, x, y]);
            // It is assumed the operation poly outputs 1 where there is a valid gate
            dbg!(&layer_proving_info_with_subset[0][*x]);
            dbg!(&layer_proving_info_with_subset[layer_index][*y]);
        }
    }

    for layer_index in 0..circuit_info.len() {
        let layer = &circuit_info[layer_index];
        for [z, x, y] in layer {
            dbg!("circuit", [z, x, y]);
            dbg!(&evaluations[layer_id + 1][*x]);
            dbg!(&evaluations[layer_id + 1 + layer_index][*y]);
        }
    }
}

// Algorithm 6
pub fn build_agi<F: Field, E: ExtensionField<F>>(
    rb: Vec<E>,
    // Contains the rc's for each layer
    rc_s: Vec<Vec<E>>,
    // Layer proving info
    layer_proving_info: LayerProvingInfoWithSubset<F>,
    // alpha for rb
    rb_alpha: E,
    // alphas for the random linear combination of the different v_subset evaluations on the corresponding rc
    alphas: Vec<E>,
    // The total gates a layer has (total add gates + total mul gates)
    total_gates_in_layer: usize,
    // Where vi and vi_subset aligns
    cki: Vec<[usize; 2]>,
    // The ci poly for the current layer
    ci: Vec<[usize; 2]>,
) -> Vec<E> {
    let depth_from_layer = layer_proving_info.v_subsets.len();

    // TODO: Uncomment
    // assert_eq!(rc_s.len(), depth_from_layer);
    // assert_eq!(alphas.len(), depth_from_layer);
    // assert_eq!(alphas.len(), depth_from_layer);

    // TODO: Assert cki length

    let mut res = vec![E::zero(); 2 * total_gates_in_layer];

    for k in 0..layer_proving_info.v_subsets.len() {
        // let mut subset = layer_proving_info.v_subsets[k].clone();
        // if subset.len() == 1 {
        //     subset.extend(vec![F::zero()]);
        // }
        // if !subset.len().is_power_of_two() {
        //     subset.extend(vec![
        //         F::zero();
        //         subset.len().next_power_of_two() - subset.len()
        //     ]);
        // }

        let igz_for_r_k = generate_eq(&rc_s[k]);

        for [t, x] in &cki {
            res[*x] += alphas[k] * igz_for_r_k[*t];
        }

        // Get igz for rb
        let igz_for_rb = generate_eq(&rb);

        for [t, x] in &ci {
            res[*x] += rb_alpha * igz_for_rb[*t];
        }
    }

    res
}

pub fn build_virgo_ahg<F: Field, E: ExtensionField<F>>(
    layer_index: usize,
    circuit_depth: usize,
    igz: &[E],
    layer_proving_info: &LayerProvingInfoWithSubset<F>,
    total_gates_in_layer: usize,
) -> (Vec<E>, Vec<E>, Vec<E>) {
    let depth_from_layer = circuit_depth - layer_index - 1;

    // TODO: use identity for b
    let add_b_ahg = phase_one(
        igz,
        &layer_proving_info.add_subsets,
        &layer_proving_info.v_subsets,
        depth_from_layer,
        total_gates_in_layer,
    );

    let add_c_ahg = phase_one(
        igz,
        &layer_proving_info.add_subsets,
        &layer_proving_info.v_subsets,
        depth_from_layer,
        total_gates_in_layer,
    );

    let mul_ahg = phase_one(
        igz,
        &layer_proving_info.mul_subsets,
        &layer_proving_info.v_subsets,
        depth_from_layer,
        total_gates_in_layer,
    );

    (add_b_ahg, add_c_ahg, mul_ahg)
}

pub fn phase_one<F: Field, E: ExtensionField<F>>(
    igz: &[E],
    f1: &Vec<Vec<[usize; 3]>>,
    vi_subset: &Vec<Vec<F>>,
    depth_from_layer: usize,
    total_gates_in_layer: usize,
) -> Vec<E> {
    // The total number of inputs to a layer cant be more than (2 * total gates in layer)
    // since the circuit is fan in 2
    let mut res = vec![E::zero(); 2 * total_gates_in_layer];

    assert_eq!(f1.len(), depth_from_layer);

    for layer_index in 0..f1.len() {
        for [z, x, y] in &f1[layer_index] {
            // It is assumed the operation poly outputs 1 where there is a valid gate
            res[*x] += igz[*z] * vi_subset[layer_index][*y];
        }
    }

    res
}

pub(crate) fn generate_circuit_info_for_cki(
    circuit: &GeneralCircuit,
    layer_id: usize,
) -> (Vec<Vec<[usize; 3]>>, Vec<Vec<[usize; 3]>>) {
    let mut add_mle = vec![vec![]; circuit.layers.len() + 1];
    let mut mul_mle = vec![vec![]; circuit.layers.len() + 1];

    let layer = &circuit.layers[layer_id];

    for i in 0..layer.gates.len() {
        let gate = &layer.gates[i];
        if gate.op == GateOp::Add {
            // Since the left input will always come from the immediate layer below
            // what we actually care about is the layer of the right input
            add_mle[gate.inputs[1].0].push([i, gate.inputs[0].1, gate.inputs[1].1]);
        }
        if gate.op == GateOp::Mul {
            mul_mle[gate.inputs[1].0].push([i, gate.inputs[0].1, gate.inputs[1].1]);
        }
    }

    (
        add_mle[layer_id + 1..].to_vec(),
        mul_mle[layer_id + 1..].to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use libra::utils::generate_eq;
    use p3_field::{AbstractField, extension::BinomialExtensionField};
    use p3_goldilocks::Goldilocks;

    use crate::{
        circuit::test::circuit_1,
        util::{build_cki, build_virgo_ahg},
    };

    use super::build_agi;

    #[test]
    fn test_n_to_1_folding() {
        // Build circuit
        let circuit = circuit_1();

        // Evaluate circuit on input
        let layer_evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        assert_eq!(
            layer_evaluations[0],
            [9, 121]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>()
        );

        // Generate sumcheck eqn for layer 1
        let layer_index = 1;
        let total_gates_in_layer = 2;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let layer_evaluation = &layer_evaluations[layer_index];

        let proving_info_with_subsets = layer_proving_info.extract_subsets(&layer_evaluations);

        let igz = generate_eq(
            &[3_usize]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        let virgo_ahg = build_virgo_ahg(
            layer_index,
            4,
            &igz,
            &proving_info_with_subsets,
            total_gates_in_layer,
        );

        dbg!(&virgo_ahg);
    }

    #[test]
    fn test_build_agi() {
        // Build circuit
        let circuit = circuit_1();

        // Evaluate circuit on input
        let layer_evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        // Generate sumcheck eqn for layer 1
        let layer_index = 0;
        let total_gates_in_layer = 4;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let proving_info_with_subsets = layer_proving_info
            .clone()
            .extract_subsets(&layer_evaluations);

        // The random challenges of each layer
        let rc_s: Vec<Vec<Goldilocks>> = vec![
            vec![0_usize]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
            vec![0_usize, 1]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
            vec![0_usize, 1, 5]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        ];

        let alphas: Vec<Goldilocks> = vec![2_usize, 3, 5]
            .iter()
            .map(|val| Goldilocks::from_canonical_usize(*val))
            .collect::<Vec<Goldilocks>>();

        let rb = vec![7_usize]
            .iter()
            .map(|val| Goldilocks::from_canonical_usize(*val))
            .collect::<Vec<Goldilocks>>();

        let rb_alpha = Goldilocks::from_canonical_usize(4);

        let cki = vec![];

        let ci = vec![];

        let _ = build_agi(
            rb,
            rc_s,
            proving_info_with_subsets.clone(),
            rb_alpha,
            alphas,
            total_gates_in_layer,
            cki,
            ci,
        );

        // let v = circuit.generate_circuit_info_for_cki(layer_index);
        let v = build_cki::<Goldilocks, BinomialExtensionField<Goldilocks, 2>>(
            // 0 = add, 1 = mul
            circuit,
            layer_index,
            layer_evaluations,
        );

        dbg!(&v);
    }
}
