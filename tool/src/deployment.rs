use crate::wallet::Wallet;
use ckb_jsonrpc_types::Script;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
    H256,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellLocation {
    OutPoint { tx_hash: H256, index: u32 },
    File { file: PathBuf },
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Cell {
    name: String,
    location: CellLocation,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DepGroup {
    name: String,
    cells: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub lock: Script,
    pub cells: Vec<Cell>,
    pub dep_groups: Vec<DepGroup>,
}

pub struct Deployment {
    wallet: Wallet,
}

impl Deployment {
    pub fn new(wallet: Wallet) -> Self {
        Deployment { wallet }
    }

    pub fn process(&self, config: DeploymentConfig) {
        let cells = &config.cells;
        let tx = self.build_cell_deploy_tx(cells);
        // send tx
        self.wallet.rpc_client().send_transaction(tx.data().into());
        // build map cell name -> out point
        let mut cells_map = HashMap::default();
        let mut i = 0;
        for cell in cells {
            match cell.location.to_owned() {
                CellLocation::OutPoint { tx_hash, index } => {
                    cells_map.insert(cell.name.to_owned(), (tx_hash, index));
                }
                CellLocation::File { .. } => {
                    cells_map.insert(cell.name.to_owned(), (tx.hash().unpack(), i));
                    i += 1;
                }
            }
        }
        let dep_groups = &config.dep_groups;
        let tx = self.build_dep_groups_deploy_tx(cells_map, dep_groups);
        // send tx
        self.wallet.rpc_client().send_transaction(tx.data().into());
    }

    fn build_cell_deploy_tx(&self, cells: &[Cell]) -> TransactionView {
        let mut cell_data: Vec<Bytes> = Vec::new();
        let mut outputs: Vec<CellOutput> = Vec::new();
        let lock = self.wallet.generate_lock();
        let mut capacity = 0;
        for cell in cells {
            match cell.location.to_owned() {
                CellLocation::OutPoint { .. } => {}
                CellLocation::File { file } => {
                    let mut data = Vec::new();
                    fs::File::open(file)
                        .expect("open")
                        .read_to_end(&mut data)
                        .expect("read");
                    let data_len = data.len();
                    cell_data.push(data.into());
                    let output = CellOutput::new_builder()
                        .lock(lock.clone())
                        .build_exact_capacity(Capacity::bytes(data_len).expect("bytes"))
                        .expect("build");
                    outputs.push(output);
                    capacity += data_len;
                }
            }
        }
        let live_cells = self
            .wallet
            .find_live_cells(Capacity::bytes(capacity).expect("capacity"));
        let inputs: Vec<_> = live_cells
            .into_iter()
            .map(|cell| {
                let out_point = OutPoint::new_builder()
                    .tx_hash(cell.created_by.tx_hash.pack())
                    .index((cell.created_by.index.value() as u32).pack())
                    .build();
                CellInput::new_builder().previous_output(out_point).build()
            })
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs.pack())
            .outputs(outputs.pack())
            .outputs_data(cell_data.pack())
            .build();
        self.wallet.sign_tx(&tx)
    }

    fn build_dep_groups_deploy_tx(
        &self,
        cells: HashMap<String, (H256, u32)>,
        dep_groups: &[DepGroup],
    ) -> TransactionView {
        let mut cell_data: Vec<Bytes> = Vec::new();
        let mut outputs: Vec<CellOutput> = Vec::new();
        let lock = self.wallet.generate_lock();
        let mut capacity = 0;
        for dep_group in dep_groups {
            let out_points: OutPointVec = dep_group
                .cells
                .iter()
                .map(|cell_name| {
                    let (tx_hash, index) = cells.get(cell_name).expect("get cell");
                    OutPoint::new_builder()
                        .tx_hash(tx_hash.pack())
                        .index(index.pack())
                        .build()
                })
                .pack();
            let data = out_points.as_bytes();
            let data_len = data.len();
            cell_data.push(data);
            let output = CellOutput::new_builder()
                .lock(lock.clone())
                .build_exact_capacity(Capacity::bytes(data_len).expect("bytes"))
                .expect("build");
            outputs.push(output);
            capacity += data_len;
        }
        let live_cells = self
            .wallet
            .find_live_cells(Capacity::bytes(capacity).expect("capacity"));
        let inputs: Vec<_> = live_cells
            .into_iter()
            .map(|cell| {
                let out_point = OutPoint::new_builder()
                    .tx_hash(cell.created_by.tx_hash.pack())
                    .index((cell.created_by.index.value() as u32).pack())
                    .build();
                CellInput::new_builder().previous_output(out_point).build()
            })
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs.pack())
            .outputs_data(cell_data.pack())
            .build();
        self.wallet.sign_tx(&tx)
    }
}
