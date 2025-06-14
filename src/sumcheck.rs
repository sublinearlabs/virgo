use p3_field::{ExtensionField, Field};
use poly::Fields;
use sum_check::primitives::SumCheckProof;

use crate::util::LayerProvingInfoWithSubset;

// what are  the basic steps I need?
// the goal is to prove the sumcheck relation for a Layer
// the layer proving info with subset should contain all information needed to do this
// hence I should be able to write a function that takes just that and returns a sumcheck proof
//
// now I need a function that represents phase 1

// TODO: add proper documentation
fn prove_phase_one<F: Field, E: ExtensionField<F>>(
    layer_proving_info: &LayerProvingInfoWithSubset<Fields<F, E>>,
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
    todo!()
}

// TODO: add proper documentation
fn build_product_bookkeeping_table<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    subsets: Vec<Vec<Fields<F, E>>>,
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
