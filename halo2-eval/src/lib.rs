// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use halo2curves::bn256::Fr;

#[derive(Clone, Debug)]
pub struct MulConfig {
    pub a: Column<Advice>,
    pub b: Column<Advice>,
    pub c: Column<Advice>,
    pub instance: Column<Instance>,
    pub q_mul: Selector,
}

#[derive(Clone, Debug, Default)]
pub struct MulCircuit {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
}

impl Circuit<Fr> for MulCircuit {
    type Config = MulConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self { a: None, b: None }
    }

    fn configure(cs: &mut ConstraintSystem<Fr>) -> Self::Config {
        let a = cs.advice_column();
        let b = cs.advice_column();
        let c = cs.advice_column();
        let instance = cs.instance_column();
        let q_mul = cs.selector();

        cs.enable_equality(a);
        cs.enable_equality(b);
        cs.enable_equality(c);
        cs.enable_equality(instance);

        cs.create_gate("a*b=c", |meta| {
            let q = meta.query_selector(q_mul);
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_advice(c, Rotation::cur());
            vec![q * (a * b - c)]
        });

        MulConfig { a, b, c, instance, q_mul }
    }

    fn synthesize(&self, cfg: MulConfig, mut layouter: impl Layouter<Fr>) -> Result<(), Error> {
        let mut c_cell_out = None;
        layouter.assign_region(
            || "mul region",
            |mut region| {
                cfg.q_mul.enable(&mut region, 0)?;

                let a_cell = region.assign_advice(
                    || "a",
                    cfg.a,
                    0,
                    || Value::known(self.a.unwrap_or_else(|| Fr::from(3u64))),
                )?;
                let b_cell = region.assign_advice(
                    || "b",
                    cfg.b,
                    0,
                    || Value::known(self.b.unwrap_or_else(|| Fr::from(5u64))),
                )?;
                let c_val = a_cell.value().copied() * b_cell.value().copied();
                let c_cell = region.assign_advice(|| "c", cfg.c, 0, || c_val)?;
                c_cell_out = Some(c_cell.cell());
                Ok(())
            },
        )?;

        // Constrain c to equal public instance (outside the region)
        layouter.constrain_instance(c_cell_out.expect("c cell"), cfg.instance, 0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;

    #[test]
    fn test_mul_mockprover() {
        let k = 6; // 2^6 rows
        let a = Fr::from(3u64);
        let b = Fr::from(5u64);
        let c = a * b;

        let circuit = MulCircuit { a: Some(a), b: Some(b) };
        let public_inputs = vec![vec![c]];

        let prover = MockProver::run(k, &circuit, public_inputs).unwrap();
        prover.assert_satisfied();
    }
}
