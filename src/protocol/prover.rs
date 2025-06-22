use p3_field::{ExtensionField, Field, PackedValue, PrimeField32};
use poly::{mle::MultilinearPoly, Fields, MultilinearExtension};
use transcript::Transcript;

use crate::circuit::GeneralCircuit;

use super::VirgoProof;
use crate::util::Subclaim;

pub fn prove<F: Field + PrimeField32, E: ExtensionField<F>>(
    circuit: &GeneralCircuit,
    evaluations: &[Vec<Fields<F, E>>],
    transcript: &mut Transcript<F, E>,
) -> VirgoProof<F, E> {
    let layer_subclaims: Vec<Vec<Subclaim<F, E>>> = vec![vec![]; circuit.layers.len()];

    // commit the output mle to the transcript
    let output_mle =
        MultilinearPoly::new_extend_to_power_of_two(evaluations[0].clone(), Fields::from_u32(0));
    output_mle.commit_to_transcript(transcript);

    // sample challenges for the output
    let r = transcript.sample_n_challenges(output_mle.num_vars());

    // what to do after sampling?
    // we want to get the claim via evaluation
    // what does the prover really care about in a subclaim?
    // only the challenges I believe i.e r
    // it needs the eval tho to generate the sumcheck claimed sum
    // even tho that feels quite useless
    let m = output_mle.evaluate(r.as_slice());

    todo!()
}

// TODO: add documentation
fn deposit_into_subset_info<T>(subset_info: &mut [Vec<T>], data: Vec<T>) {
    debug_assert_eq!(subset_info.len() + 1, data.len());

    let mut data_iter = data.into_iter();
    subset_info[0].push(data_iter.next().unwrap());

    for (entry, data) in subset_info.iter_mut().zip(data_iter) {
        entry.push(data);
    }
}

#[cfg(test)]
mod test {
    use super::deposit_into_subset_info;

    #[test]
    fn test_deposit_subset_info() {
        let mut subset_info = vec![vec![]; 3];

        deposit_into_subset_info(&mut subset_info, vec![1, 2, 3, 4]);
        assert_eq!(subset_info, vec![vec![1, 2], vec![3], vec![4]]);

        deposit_into_subset_info(&mut subset_info[1..], vec![5, 6, 7]);
        assert_eq!(subset_info, vec![vec![1, 2], vec![3, 5, 6], vec![4, 7]]);
    }
}
