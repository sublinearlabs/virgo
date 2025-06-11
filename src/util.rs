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
    vi_subset_instruction: &Vec<Vec<usize>>,
    subset: &Vec<Vec<F>>,
) -> Vec<Vec<(usize, usize)>> {
    let mut res = vec![];

    for i in 0..subset.len() {
        let subset = &subset[i];
        let mut layer_res = vec![];
        for j in 0..subset.len() {
            layer_res.push((j, vi_subset_instruction[i][j]));
        }
        res.push(layer_res);
    }

    res
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
    cki: &Vec<Vec<(usize, usize)>>,
    // The ci poly for the current layer
    ci: &Vec<(usize, usize)>,

    vi_subset: &Vec<Vec<F>>,
) -> Vec<E> {
    let depth_from_layer = layer_proving_info.v_subsets.len();

    assert_eq!(rc_s.len(), depth_from_layer);
    assert_eq!(alphas.len(), depth_from_layer);
    assert_eq!(alphas.len(), depth_from_layer);
    assert_eq!(cki.len(), depth_from_layer);

    let mut res = vec![E::zero(); 2 * total_gates_in_layer];

    for k in 0..layer_proving_info.v_subsets.len() {
        let igz_for_r_k = generate_eq(&rc_s[k]);

        for (t, x) in &cki[k] {
            res[*x] += alphas[k] * igz_for_r_k[*t];
            // res[*x] += igz_for_r_k[*t] * vi_subset[k][*t] * alphas[k];
        }

        // Get igz for rb
        let igz_for_rb = generate_eq(&rb);

        for (t, x) in ci {
            res[*x] += rb_alpha * igz_for_rb[*t];
            // res[*x] += igz_for_rb[*t] * vi_subset[0][*t] * rb_alpha;
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
        let layer_index = 0;
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
    fn test_build_cki() {
        // Build circuit
        let circuit = circuit_1();

        // Evaluate circuit on input
        let layer_evaluations = circuit.eval(
            &[1, 2, 3, 4, 5, 6]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        );

        // Generate sumcheck eqn for layer 0
        let layer_index = 0;

        let layer_proving_info = circuit.generate_layer_proving_info(layer_index);

        let proving_info_with_subsets = layer_proving_info
            .clone()
            .extract_subsets(&layer_evaluations);

        let cki = &build_cki::<Goldilocks, BinomialExtensionField<Goldilocks, 2>>(
            &layer_proving_info.v_subset_instruction,
            &proving_info_with_subsets.v_subsets,
        );

        assert_eq!(cki[0], vec![(0, 0), (1, 1)]);
        assert_eq!(cki[1], vec![(0, 3)]);
        assert_eq!(cki[2], vec![(0, 2)]);
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
        let total_gates_in_layer = 2;

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
            vec![0_usize, 1, 1]
                .iter()
                .map(|val| Goldilocks::from_canonical_usize(*val))
                .collect::<Vec<Goldilocks>>(),
        ];

        let alphas: Vec<Goldilocks> = vec![1_usize, 1, 1]
            .iter()
            .map(|val| Goldilocks::from_canonical_usize(*val))
            .collect::<Vec<Goldilocks>>();

        let rb = vec![1_usize]
            .iter()
            .map(|val| Goldilocks::from_canonical_usize(*val))
            .collect::<Vec<Goldilocks>>();

        let rb_alpha = Goldilocks::from_canonical_usize(1);

        let cki = &build_cki::<Goldilocks, BinomialExtensionField<Goldilocks, 2>>(
            &layer_proving_info.v_subset_instruction,
            &proving_info_with_subsets.v_subsets,
        );

        let ci = &cki[0];

        let agi = build_agi(
            rb,
            rc_s,
            proving_info_with_subsets.clone(),
            rb_alpha,
            alphas,
            total_gates_in_layer,
            cki,
            ci,
            &proving_info_with_subsets.v_subsets,
        );

        dbg!(&agi);
    }
}
