use std::path::Path;

use axiom_eth::util::circuit::PreCircuit;
use halo2_base::gates::builder::CircuitBuilderStage;
use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use halo2_base::utils::fs::gen_srs;
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
#[allow(unused_imports)]
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_scaffold::scaffold::cmd::{Cli, SnarkCmd};
use halo2_scaffold::scaffold::{pre_run_builder_on_inputs, ScaffoldCircuitBuilder};
use serde::{Deserialize, Serialize};
use snark_verifier_sdk::evm::{evm_verify, gen_evm_proof_shplonk, gen_evm_verifier_shplonk};
use snark_verifier_sdk::{gen_pk, CircuitExt};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub x: String,
}

fn some_algorithm_in_zk<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    let x = F::from_str_vartime(&input.x).expect("deserialize field element should not fail");

    let x = ctx.load_witness(x);

    make_public.push(x);

    let gate = GateChip::<F>::default();

    let c = F::from(72);

    // x^2 + 72
    let out = gate.mul_add(ctx, x, x, Constant(c));

    make_public.push(out);

    println!("x: {:?}", x.value());
    println!("out: {:?}", out.value());

    assert_eq!(*x.value() * x.value() + c, *out.value());
}

fn main() {
    let cli = Cli {
        command: SnarkCmd::Mock,
        name: "txn_circuit".to_string(),
        degree: 8,
        input_path: None,
        create_contract: false,
        config_path: None,
        data_path: None,
    };

    let k = cli.degree;

    let params = gen_srs(k);

    let private_inputs = CircuitInput {
        x: "12".to_string(),
    };

    let txn_precircuit = pre_run_builder_on_inputs(
        |builder, inp, public| some_algorithm_in_zk(builder.main(0), inp, public),
        private_inputs,
    );

    let txn_circuit = txn_precircuit.create_circuit(CircuitBuilderStage::Mock, None, &params);

    let txn_pk = gen_pk(&params, &txn_circuit, None);

    let num_instances = txn_circuit.num_instance();
    let instances = txn_circuit.instances();

    let deployment_code = gen_evm_verifier_shplonk::<ScaffoldCircuitBuilder<Fr>>(
        &params,
        txn_pk.get_vk(),
        num_instances,
        Some(Path::new("txn.yul")),
    );

    let evm_proof = gen_evm_proof_shplonk(&params, &txn_pk, txn_circuit, instances.clone());

    evm_verify(deployment_code, instances, evm_proof);
}
