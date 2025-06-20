use p3_field::{ExtensionField, Field};
use poly::Fields;

use crate::circuit::GeneralCircuit;

use super::VirgoProof;

pub fn prove<F: Field, E: ExtensionField<F>>(
    circuit: &GeneralCircuit,
    evaluations: &[Vec<Fields<F, E>>],
) -> VirgoProof<F, E> {
    todo!()
}
