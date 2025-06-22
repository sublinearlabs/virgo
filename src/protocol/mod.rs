pub mod prover;
pub mod sumcheck;

use p3_field::{ExtensionField, Field};
use poly::Fields;
use sum_check::primitives::SumCheckProof;

type LayerSumcheck<F, E> = (SumCheckProof<F, E>, Vec<Fields<F, E>>);
type FoldingSumcheck<F, E> = (SumCheckProof<F, E>, Fields<F, E>);

pub struct VirgoProof<F: Field, E: ExtensionField<F>> {
    pub layer_sumchecks: Vec<LayerSumcheck<F, E>>,
    pub folding_sumchecks: Vec<FoldingSumcheck<F, E>>,
}
