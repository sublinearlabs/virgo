use std::rc::Rc;

use p3_field::{ExtensionField, Field, PrimeField32};
use poly::{
    mle::MultilinearPoly,
    utils::{generate_eq, product_poly},
    vpoly::VPoly,
    Fields, MultilinearExtension,
};
use sum_check::{
    padded_sumcheck::PaddedSumcheck, primitives::SumCheckProof, sumcheckable::Sumcheckable,
};
use transcript::Transcript;

use crate::util::LayerProvingInfoWithSubset;

pub(crate) fn prove_phase_two<F: Field + PrimeField32, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    phase_one_challenges: &[Fields<F, E>],
    layer_proving_info: &LayerProvingInfoWithSubset<Fields<F, E>>,
    transcript: &mut Transcript<F, E>,
) -> SumCheckProof<F, E> {
    let iux = generate_eq(phase_one_challenges);

    let subset_lens = layer_proving_info
        .v_subsets
        .iter()
        .map(|vi| vi.len())
        .collect::<Vec<_>>();

    let constant = MultilinearPoly::new_extend_to_power_of_two(
        layer_proving_info.v_subsets[0].clone(),
        Fields::Base(F::zero()),
    )
    .evaluate(phase_one_challenges);

    // generate the bookkeeping tables
    let add_tables_with_constant = build_bookkeeping_tables(
        igz,
        &iux,
        &layer_proving_info.add_subsets,
        &constant,
        &subset_lens,
    );

    let add_tables_with_identity = build_bookkeeping_tables_with_identity(
        igz,
        &iux,
        &layer_proving_info.add_subsets,
        &subset_lens,
    );

    let mul_tables = build_bookkeeping_tables(
        igz,
        &iux,
        &layer_proving_info.mul_subsets,
        &constant,
        &subset_lens,
    );

    let iter_1 = add_tables_with_constant.into_iter().map(|p| {
        VPoly::new(
            vec![MultilinearPoly::new_extend_to_power_of_two(
                p,
                Fields::Base(F::zero()),
            )],
            2,
            Rc::new(|evals: &[Fields<F, E>]| evals[0]),
        )
    });

    let iter_2 = add_tables_with_identity
        .into_iter()
        .zip(&layer_proving_info.v_subsets)
        .map(|(p, subset)| {
            product_poly(vec![
                MultilinearPoly::new_extend_to_power_of_two(p, Fields::Base(F::zero())),
                MultilinearPoly::new_extend_to_power_of_two(
                    subset.to_vec(),
                    Fields::Base(F::zero()),
                ),
            ])
        });

    let iter_3 = mul_tables
        .into_iter()
        .zip(&layer_proving_info.v_subsets)
        .map(|(p, subset)| {
            product_poly(vec![
                MultilinearPoly::new_extend_to_power_of_two(p, Fields::Base(F::zero())),
                MultilinearPoly::new_extend_to_power_of_two(
                    subset.to_vec(),
                    Fields::Base(F::zero()),
                ),
            ])
        });

    let vpolys = iter_1.chain(iter_2).chain(iter_3);

    // determine the highest number of variables
    let max_var = vpolys.clone().map(|p| p.num_vars()).max().unwrap();

    // prepare for padded sumcheck
    let mut padded_polys = vpolys
        .into_iter()
        .map(|vp| {
            let pad_count = max_var - vp.num_vars();
            PaddedSumcheck::new(vp, pad_count)
        })
        .collect::<Vec<_>>();

    let mut round_messages = vec![];
    let mut challenges = vec![];

    for _ in 0..max_var {
        // combine the round messages for all the padded polynomials
        let round_message = merge_round_messages(
            &padded_polys
                .iter()
                .map(|p| p.round_message())
                .collect::<Vec<_>>(),
        );
        transcript.observe_ext_element(
            &round_message
                .iter()
                .map(|val| val.to_extension_field())
                .collect::<Vec<E>>(),
        );
        let challenge = Fields::Extension(transcript.sample_challenge());
        for poly in &mut padded_polys {
            poly.receive_challenge(&challenge);
        }
        round_messages.push(round_message);
        challenges.push(challenge);
    }

    SumCheckProof {
        claimed_sum: Fields::Base(F::zero()),
        round_polynomials: round_messages,
        challenges,
    }
}

fn build_bookkeeping_tables<F: Field, E: ExtensionField<F>>(
    igz: &[Fields<F, E>],
    iux: &[Fields<F, E>],
    sparse_entries: &[Vec<[usize; 3]>],
    constant: &Fields<F, E>,
    table_lens: &[usize],
) -> Vec<Vec<Fields<F, E>>> {
    debug_assert_eq!(sparse_entries.len(), table_lens.len());
    let mut tables = vec![];

    for (sparse_entry, table_len) in sparse_entries.iter().zip(table_lens) {
        let mut table = vec![Fields::Base(F::zero()); *table_len];
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
    table_lens: &[usize],
) -> Vec<Vec<Fields<F, E>>> {
    debug_assert_eq!(sparse_entries.len(), table_lens.len());
    let mut tables = vec![];

    for (sparse_entry, table_len) in sparse_entries.iter().zip(table_lens) {
        let mut table = vec![Fields::Base(F::zero()); *table_len];
        for [z, x, y] in sparse_entry {
            table[*y] += igz[*z] * iux[*x];
        }
        tables.push(table);
    }

    tables
}

fn merge_round_messages<F: Field, E: ExtensionField<F>>(
    round_messages: &[Vec<Fields<F, E>>],
) -> Vec<Fields<F, E>> {
    let mut result = round_messages[0].clone();
    for round_message in round_messages.iter().skip(1) {
        // who is responsible for the length of the round messages??
        // the code not the use
        debug_assert_eq!(result.len(), round_message.len());
        for i in 0..result.len() {
            result[i] += round_message[i];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use p3_field::{extension::BinomialExtensionField, ExtensionField, Field};
    use p3_goldilocks::Goldilocks as F;
    use poly::Fields;

    use crate::sumcheck::phase_two::merge_round_messages;
    type E = BinomialExtensionField<F, 2>;

    fn to_fields<F: Field, E: ExtensionField<F>>(values: Vec<usize>) -> Vec<Fields<F, E>> {
        values
            .into_iter()
            .map(|v| Fields::Base(F::from_canonical_usize(v)))
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_merge_round_messages() {
        let round_messages = [
            to_fields::<F, E>(vec![1, 2]),
            to_fields(vec![3, 4]),
            to_fields(vec![3, 5]),
        ];
        assert_eq!(
            merge_round_messages(&round_messages),
            to_fields(vec![7, 11])
        );
    }
}
