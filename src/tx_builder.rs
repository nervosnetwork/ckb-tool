use super::Context;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, DepType, TransactionBuilder, TransactionView},
    packed::{CellDep, CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};
use rand::{thread_rng, Rng};

pub struct TxBuilder {
    lock_script: Bytes,
    type_script: Option<Bytes>,
    previous_output_data: Bytes,
    previous_cell: Option<Bytes>,
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
            lock_script: Script::default().as_bytes(),
            type_script: None,
            previous_output_data: Bytes::new(),
            previous_cell: None,
            input_capacity: 41,
            output_capacity: 41,
            witnesses: Vec::new(),
            outputs_data: Vec::new(),
        }
    }

    pub fn lock_script(mut self, lock_script: Bytes) -> Self {
        self.lock_script = lock_script;
        self
    }

    pub fn type_script(mut self, type_script: Bytes) -> Self {
        self.type_script = Some(type_script);
        self
    }

    pub fn previous_output_data(mut self, data: Bytes) -> Self {
        self.previous_output_data = data;
        self
    }

    pub fn previous_cell(mut self, cell: Bytes) -> Self {
        self.previous_cell = Some(cell);
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
        let lock_script = Script::new_unchecked(self.lock_script.clone());
        let lock_out_point = context
            .get_contract_out_point(&lock_script.code_hash())
            .ok_or("can't found contract by lock_data_hash")?;
        let mut output_cell = CellOutput::new_builder()
            .capacity(output_capacity.pack())
            .build();

        // setup type script
        if let Some(type_script) = self.type_script.clone() {
            output_cell = output_cell
                .as_builder()
                .type_(Some(Script::new_unchecked(type_script)).pack())
                .build();
        }
        // setup unlock script
        let cell_to_spent = match self.previous_cell.clone() {
            Some(cell) => CellOutput::new_unchecked(cell),
            None => CellOutput::new_builder()
                .capacity(input_capacity.pack())
                .lock(lock_script)
                .build(),
        };
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
        if let Some(ref type_script) = self.type_script {
            let type_script = Script::new_unchecked(type_script.clone());
            let type_out_point = context
                .get_contract_out_point(&type_script.code_hash())
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
