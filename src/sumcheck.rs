use std::rc::Rc;

use p3_field::{ExtensionField, Field, PrimeField32};
use poly::utils::generate_eq;
use poly::{Fields, mle::MultilinearPoly, vpoly::VPoly};
use sum_check::SumCheck;
use sum_check::interface::SumCheckInterface;
use sum_check::primitives::SumCheckProof;
use transcript::Transcript;

use crate::util::LayerProvingInfoWithSubset;

// what are  the basic steps I need?
// the goal is to prove the sumcheck relation for a Layer
// the layer proving info with subset should contain all information needed to do this
// hence I should be able to write a function that takes just that and returns a sumcheck proof
//
// now I need a function that represents phase 1

// TODO: add proper documentation
fn prove_phase_one<F: Field + PrimeField32, E: ExtensionField<F>>(
    claimed_sum: Fields<F, E>,
    output_point: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<Fields<F, E>>,
    transcript: &mut Transcript<F, E>,
) -> SumCheckProof<F, E> {
    // what is required to prove phase one
    // we need to generate three bookkeeping tables
    // use the vpoly to construct a single combination poly
    // run partial sumcheck on that and return the proof

    // what inputs do we need to build a bookkeeping table?
    // for libra we just need the I(g, z), f[z, b, c] and v(c)
    // for virgo we have different f_i[z, b, c] and v_(c) pairs
    // what should one call this subroutine?
    // build_product_bookkeping_table()

    // build the I(g, z) table first
    //let igz = generate_eq::<F, E>();
    let igz = generate_eq(output_point);

    // TODO: document this section
    let add_b_ahg = build_bookkeeping_table_with_identity(
        &igz,
        &layer_proving_info.add_subsets,
        layer_proving_info.v_subsets[0].len(),
    );

    // TODO: document this section
    let add_c_ahg = build_bookkeeping_table(
        &igz,
        &layer_proving_info.add_subsets,
        &layer_proving_info.v_subsets,
    );

    // TODO: document this section
    let mul_ahg = build_bookkeeping_table(
        &igz,
        &layer_proving_info.mul_subsets,
        &layer_proving_info.v_subsets,
    );

    let mles = [
        add_b_ahg,
        add_c_ahg,
        mul_ahg,
        layer_proving_info.v_subsets[0].clone(),
    ]
    .into_iter()
    .map(|p| MultilinearPoly::new_extend_to_power_of_two(p, Fields::Base(F::zero())))
    .collect();

    // build the vpoly
    let mut poly = VPoly::new(mles, 2, Rc::new(|evals: &[Fields<F, E>]| evals[0]));

    SumCheck::prove_partial(claimed_sum, &mut poly, transcript).unwrap()
}

// TODO: add proper documentation
fn build_bookkeeping_table<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    subsets: &[Vec<Fields<F, E>>],
) -> Vec<Fields<F, E>> {
    // ensure there is one sparse entry for each subset
    debug_assert_eq!(sparse_entries.len(), subsets.len());

    // we need something in the size of x, how do we know the size of x?
    // is that the same as the size of the first subset??

    // the size of the table is based on the size of the first subset vector
    // as the first subset vector is also the common vector for all layers
    let mut table = vec![Fields::Base(F::zero()); subsets[0].len()];

    // next we need to iterate over each (entry, subset) pair
    // and use that to populate the table
    for (sparse_entry, subset) in sparse_entries.iter().zip(subsets) {
        for [z, x, y] in sparse_entry {
            table[*x] += igz[*z] * subset[*y];
        }
    }

    table
}

// TODO: add documentation
fn build_bookkeeping_table_with_identity<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    table_len: usize,
) -> Vec<Fields<F, E>> {
    let mut table = vec![Fields::Base(F::zero()); table_len];
    for sparse_entry in sparse_entries {
        for [z, x, _] in sparse_entry {
            table[*x] += igz[*z]
        }
    }
    table
}

/// Utility function to merge two sumcheck proofs
/// used to merge the phase 1 and phase 2 sumcheck proofs
fn merge_sumcheck_proofs<F: Field, E: ExtensionField<F>>(
    proofs: [SumCheckProof<F, E>; 2],
) -> SumCheckProof<F, E> {
    let [proof1, proof2] = proofs;
    SumCheckProof {
        claimed_sum: proof1.claimed_sum,
        round_polynomials: [proof1.round_polynomials, proof2.round_polynomials].concat(),
        challenges: [proof1.challenges, proof2.challenges].concat(),
    }
}
