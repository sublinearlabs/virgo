pub mod prover;
pub mod sumcheck;
pub mod verifier;

use p3_field::{ExtensionField, Field};
use poly::Fields;
use sum_check::primitives::SumCheckProof;

type LayerSumcheck<F, E> = (SumCheckProof<F, E>, Vec<Fields<F, E>>);
type FoldingSumcheck<F, E> = (SumCheckProof<F, E>, Fields<F, E>);

#[derive(Default)]
pub struct VirgoProof<F: Field, E: ExtensionField<F>> {
    pub(crate) layer_sumchecks: Vec<LayerSumcheck<F, E>>,
    pub(crate) folding_sumchecks: Vec<FoldingSumcheck<F, E>>,
}

impl<F: Field, E: ExtensionField<F>> VirgoProof<F, E> {
    fn add_layer_proof(
        &mut self,
        layer_proof: SumCheckProof<F, E>,
        layer_hints: Vec<Fields<F, E>>,
    ) {
        self.layer_sumchecks.push((layer_proof, layer_hints))
    }

    fn add_folding_proof(
        &mut self,
        folding_proof: SumCheckProof<F, E>,
        folding_hint: Fields<F, E>,
    ) {
        self.folding_sumchecks.push((folding_proof, folding_hint))
    }
}
