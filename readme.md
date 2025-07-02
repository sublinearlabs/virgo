# Virgo++: Interactive Proofs for General Arithmetic Circuits

This project implements the interactive proof protocol from the paper "Doubly Efficient Interactive Proofs for General Arithmetic Circuits with Linear Prover Time" by Jiaheng Zhang et al. Virgo++ extends the capabilities of the GKR protocol to handle general (arbitrary) arithmetic circuits with a prover time that is linear in the circuit size.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Usage

To use Virgo++ in your Rust project, add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
virgo = { path = "../path/to/virgo" } # or specify a version or git URL
```

Below are the steps to import the library, create a circuit, run the prover, and verify the proof. The example computes `(a + b) * (c + d)` for inputs `a=1, b=2, c=3, d=4`.

### Step 1: Import Required Modules

Import the necessary modules and types from the Virgo library, along with required dependencies for field arithmetic.

```rust
use virgo::circuit_builder::Builder;
use virgo::circuit::{GateOp, GeneralCircuit};
use virgo::protocol::prover::prove;
use virgo::protocol::verifier::verify;
use virgo::transcript::Transcript;
use p3_field::extension::BinomialExtensionField;
use p3_mersenne_31::Mersenne31 as F;
use poly::Fields;
type E = BinomialExtensionField<F, 3>;
```

### Step 2: Create a Circuit

Use the `Builder` to construct a general arithmetic circuit. This example creates a circuit that computes `(a + b) * (c + d)`.

```rust
let mut builder = Builder::init();
let a = builder.create_input_node();
let b = builder.create_input_node();
let c = builder.create_input_node();
let d = builder.create_input_node();
let sum1 = builder.add_node(a, b, &GateOp::Add); // a + b
let sum2 = builder.add_node(c, d, &GateOp::Add); // c + d
let product = builder.add_node(sum1, sum2, &GateOp::Mul); // (a + b) * (c + d)
let circuit = builder.build_circuit();
```

### Step 3: Evaluate the Circuit

Provide inputs and evaluate the circuit to obtain the layer evaluations, which are used in proving and verification.

```rust
let inputs = Fields::<F, E>::from_u32_vec(vec![1, 2, 3, 4]); // a=1, b=2, c=3, d=4
let evaluations = circuit.eval(&inputs);
```

### Step 4: Generate a Proof

Initialize a transcript and use the `prove` function to generate a `VirgoProof` for the circuit's evaluation.

```rust
let mut transcript = Transcript::<F, E>::init();
let proof = prove(&circuit, &evaluations, &mut transcript);
```

### Step 5: Verify the Proof

Use the `verify` function to check the proof's validity against the circuit, inputs, and output evaluations.

```rust
let mut verifier_transcript = Transcript::<F, E>::init();
let is_valid = verify(
    &circuit,
    &proof,
    &inputs,
    &evaluations[0], // Output layer evaluations
    &mut verifier_transcript,
).expect("Verification failed");
assert!(is_valid, "Proof verification failed");
```