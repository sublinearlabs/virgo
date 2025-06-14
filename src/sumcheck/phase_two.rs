use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{utils::generate_eq, Fields};
use sum_check::primitives::SumCheckProof;
use transcript::Transcript;

use crate::util::LayerProvingInfoWithSubset;

// how do we handle phase 2??
// we need the first set of challenges
// then we evaluate the first subset at that challenge point
// this will give us our scalar
// what are we left with after evaluating at the challenge points?
// w(b) was factor so that turns to a scalar
// we had three groups
// w(b) * add_b + w(b) * mul_b * w(c) + add_c * w(c)
// basically we have a series of polynomials some singular some product
// all summed and they might have different number of variables
// I think I need to build the product poly's and the singular multiplinear polys
// then find the max number of variables
// and then convert each one to a padded polynomial
// run sumcheck stepwise on them
// I might want everything to be a v poly because of the max var degree issue
// we want all the round poly's to be of the same size so we can do the element wise summation

// TODO: create hackmd describing the phase two analysis
pub(crate) fn prove_phase_two<F: Field + PrimeField32, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    phase_one_challenges: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<Fields<F, E>>,
    transcript: &mut Transcript<F, E>,
) -> SumCheckProof<F, E> {
    // I need a function that can generate the tables
    // should take Igz and Iux
    let iux = generate_eq(phase_one_challenges);

    let subset_lens = layer_proving_info
        .v_subsets
        .iter()
        .map(|vi| vi.len())
        .collect::<Vec<_>>();

    // need to create a function that can build the bookkeeping table
    todo!()
}

// what does this require?
fn build_bookkeeping_tables<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    iux: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    constant: &Fields<F, E>,
    table_lens: Vec<usize>,
) -> Vec<Vec<Fields<F, E>>> {
    // we are not building the table the conventional way
    // how do we figure out the length of each table??
    // the table length should be based on the size of c
    // what about the table that doesn't make use of c at all
    //
    // table[y] += Igz[g] * Iux[x] * constant
    // I guess we can get the table length from the corresponding subsets

    debug_assert_eq!(sparse_entries.len(), table_lens.len());
    let mut tables = vec![];

    // what do we iterate over??
    // has to be sparse entries
    for (sparse_entry, table_len) in sparse_entries.iter().zip(table_lens) {
        let mut table = vec![Fields::Base(F::zero()); table_len];
        for [z, x, y] in sparse_entry {
            table[*y] += igz[*z] * iux[*x] * *constant;
        }
        tables.push(table);
    }

    tables
}

fn build_bookkeeping_tables_with_identity<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    iux: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    table_lens: Vec<usize>,
) -> Vec<Vec<Fields<F, E>>> {
    debug_assert_eq!(sparse_entries.len(), table_lens.len());
    let mut tables = vec![];

    // what do we iterate over??
    // has to be sparse entries
    for (sparse_entry, table_len) in sparse_entries.iter().zip(table_lens) {
        let mut table = vec![Fields::Base(F::zero()); table_len];
        for [z, x, y] in sparse_entry {
            table[*y] += igz[*z] * iux[*x];
        }
        tables.push(table);
    }

    tables
}
