pub mod sumcheck;

use p3_field::{ExtensionField, Field};
use poly::Fields;
use sum_check::primitives::SumCheckProof;

pub struct VirgoProof<F: Field, E: ExtensionField<F>> {
    pub layer_sumchecks: (SumCheckProof<F, E>, Vec<Fields<F, E>>),
    pub folding_sumchecks: (SumCheckProof<F, E>, Fields<F, E>),
}
