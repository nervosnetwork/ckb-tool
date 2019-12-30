use super::Context;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, DepType, ScriptHashType, TransactionBuilder, TransactionView},
    packed::{CellDep, CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};
use rand::{thread_rng, Rng};

pub struct TxBuilder {
    lock_bin: Bytes,
    type_bin: Option<Bytes>,
    previous_output_data: Bytes,
    input_capacity: u64,
    output_capacity: u64,
    witnesses: Vec<Bytes>,
    outputs_data: Vec<Bytes>,
}

impl Default for TxBuilder {
    fn default() -> Self {
        TxBuilder::new()
    }
}

impl TxBuilder {
    pub fn new() -> TxBuilder {
        TxBuilder {
            lock_bin: Bytes::new(),
            type_bin: None,
            previous_output_data: Bytes::new(),
            input_capacity: 41,
            output_capacity: 41,
            witnesses: Vec::new(),
            outputs_data: Vec::new(),
        }
    }

    pub fn lock_bin(mut self, lock_bin: Bytes) -> Self {
        self.lock_bin = lock_bin;
        self
    }

    pub fn type_bin(mut self, type_bin: Bytes) -> Self {
        self.type_bin = Some(type_bin);
        self
    }

    pub fn previous_output_data(mut self, data: Bytes) -> Self {
        self.previous_output_data = data;
        self
    }

    pub fn input_capacity(mut self, capacity: u64) -> Self {
        self.input_capacity = capacity;
        self
    }

    pub fn output_capacity(mut self, capacity: u64) -> Self {
        self.output_capacity = capacity;
        self
    }

    pub fn witnesses(mut self, witnesses: Vec<Bytes>) -> Self {
        self.witnesses = witnesses;
        self
    }

    pub fn outputs_data(mut self, outputs_data: Vec<Bytes>) -> Self {
        self.outputs_data = outputs_data;
        self
    }

    pub fn inject_and_build(&mut self, context: &mut Context) -> Result<TransactionView, &str> {
        let input_capacity = Capacity::shannons(self.input_capacity);
        let output_capacity = Capacity::shannons(self.output_capacity);

        let mut rng = thread_rng();
        let previous_tx_hash = {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            buf.pack()
        };
        let previous_index = 0;
        let previous_out_point = OutPoint::new(previous_tx_hash, previous_index);
        let lock_data_hash = CellOutput::calc_data_hash(&self.lock_bin);
        let lock_out_point = context
            .get_contract_out_point(&lock_data_hash)
            .ok_or("can't found contract by lock_data_hash")?;
        // setup unlock script
        let lock_script = Script::new_builder()
            .code_hash(lock_data_hash)
            .hash_type(ScriptHashType::Data.into())
            .build();
        let cell_to_spent = CellOutput::new_builder()
            .capacity(input_capacity.pack())
            .lock(lock_script)
            .build();
        let mut output_cell = CellOutput::new_builder()
            .capacity(output_capacity.pack())
            .build();

        // setup type script
        if let Some(ref type_bin) = self.type_bin.clone() {
            let type_data_hash = CellOutput::calc_data_hash(type_bin);
            let type_script = Script::new_builder()
                .code_hash(type_data_hash)
                .hash_type(ScriptHashType::Data.into())
                .build();
            output_cell = output_cell
                .as_builder()
                .type_(Some(type_script).pack())
                .build();
        }
        context.insert_cell(
            previous_out_point.clone(),
            cell_to_spent,
            self.previous_output_data.to_owned().into(),
        );
        let mut tx_builder = TransactionBuilder::default()
            .input(CellInput::new(previous_out_point, 0))
            .cell_dep(
                CellDep::new_builder()
                    .out_point(lock_out_point.to_owned())
                    .dep_type(DepType::Code.into())
                    .build(),
            )
            .output(output_cell)
            .output_data(Bytes::new().pack());
        if let Some(ref type_bin) = self.type_bin {
            let type_data_hash = CellOutput::calc_data_hash(type_bin);
            let type_out_point = context
                .get_contract_out_point(&type_data_hash)
                .ok_or("can't found contract by type_data_hash")?;
            tx_builder = tx_builder.cell_dep(
                CellDep::new_builder()
                    .out_point(type_out_point.to_owned())
                    .dep_type(DepType::Code.into())
                    .build(),
            );
        }

        let witnesses = self
            .witnesses
            .clone()
            .into_iter()
            .map(|wit| Bytes::from(wit).pack())
            .collect();
        let outputs_data = self
            .outputs_data
            .clone()
            .into_iter()
            .map(|data| Bytes::from(data).pack())
            .collect();
        let tx = tx_builder
            .set_witnesses(witnesses)
            .set_outputs_data(outputs_data)
            .build();
        Ok(tx)
    }
}
