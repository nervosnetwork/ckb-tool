use crate::wallet::Wallet;
use ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::Script;
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed,
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
    GenesisCellbase { index: u32 },
    File { file: PathBuf },
}

impl CellLocation {
    fn is_on_chain(&self) -> bool {
        match self {
            CellLocation::File { .. } => false,
            _ => true,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub name: String,
    pub location: CellLocation,
    pub create_type_id: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DepGroup {
    pub name: String,
    pub cells: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub lock: Script,
    pub cells: Vec<Cell>,
    pub dep_groups: Vec<DepGroup>,
}

#[derive(Default)]
pub struct DeploymentContext {
    pub cells_deploy_tx_hash: H256,
    pub dep_groups_deploy_tx_hash: H256,
    pub cells: Vec<(Cell, u32, Option<H256>)>,
    pub dep_groups: Vec<(DepGroup, u32)>,
}

pub struct Deployment {
    wallet: Wallet,
    tx_fee: Capacity,
}

impl Deployment {
    pub fn new(wallet: Wallet) -> Self {
        // TODO optimize tx fee
        let tx_fee = Capacity::bytes(100).expect("fee");
        Deployment { wallet, tx_fee }
    }

    pub fn process(&mut self, config: DeploymentConfig) -> DeploymentContext {
        let cells = &config.cells;
        let mut context = DeploymentContext::default();
        let mut cells_map = HashMap::with_capacity(cells.len());
        let cell_deploy_tx =
            self.build_cell_deploy_tx(&mut context, cells, config.lock.clone().into());
        if let Some(tx) = cell_deploy_tx.as_ref() {
            // send tx
            self.wallet.send_transaction(tx);
            context.cells_deploy_tx_hash = tx.hash().unpack();
            println!("send transaction {:#x}", context.cells_deploy_tx_hash);
        }
        let cell_deploy_tx_hash: Option<H256> =
            cell_deploy_tx.as_ref().map(|tx| tx.hash().unpack());
        // build map cell name -> out point
        let mut i = 0;
        for cell in cells {
            match cell.location.to_owned() {
                CellLocation::OutPoint { tx_hash, index } => {
                    cells_map.insert(cell.name.to_owned(), (tx_hash, index));
                }
                CellLocation::GenesisCellbase { index } => {
                    let tx_hash = self.wallet.metadata().genesis_cellbase_tx_hash.clone();
                    cells_map.insert(cell.name.to_owned(), (tx_hash, index));
                }
                CellLocation::File { .. } => {
                    cells_map.insert(
                        cell.name.to_owned(),
                        (
                            cell_deploy_tx_hash
                                .as_ref()
                                .expect("no cell deploy tx")
                                .to_owned(),
                            i,
                        ),
                    );
                    i += 1;
                }
            }
        }
        let dep_groups = &config.dep_groups;
        let tx = self.build_dep_groups_deploy_tx(
            &mut context,
            cells_map,
            dep_groups,
            config.lock.into(),
        );
        // send tx
        self.wallet.send_transaction(&tx);
        context.dep_groups_deploy_tx_hash = tx.hash().unpack();
        println!("send transaction {:#x}", context.dep_groups_deploy_tx_hash);
        context
    }

    fn build_cell_deploy_tx(
        &self,
        context: &mut DeploymentContext,
        cells: &[Cell],
        lock: packed::Script,
    ) -> Option<TransactionView> {
        let mut cell_data: Vec<Bytes> = Vec::new();
        let mut capacity = 0;
        for cell in cells {
            match cell.location.to_owned() {
                CellLocation::OutPoint { .. } => {}
                CellLocation::GenesisCellbase { .. } => {}
                CellLocation::File { file } => {
                    let mut data = Vec::new();
                    fs::File::open(file)
                        .expect("open")
                        .read_to_end(&mut data)
                        .expect("read");
                    let data_len = data.len();
                    cell_data.push(data.into());
                    capacity += data_len;
                }
            }
        }
        if cell_data.is_empty() {
            return None;
        }
        let live_cells = self
            .wallet
            .find_live_cells(Capacity::bytes(capacity).expect("capacity"), self.tx_fee);
        let inputs: Vec<_> = live_cells
            .into_iter()
            .map(|cell| {
                let out_point = packed::OutPoint::new_builder()
                    .tx_hash(cell.created_by.tx_hash.pack())
                    .index((cell.created_by.index.value() as u32).pack())
                    .build();
                packed::CellInput::new_builder()
                    .previous_output(out_point)
                    .build()
            })
            .collect();
        let outputs: Vec<_> = cell_data
            .iter()
            .zip(cells.iter().filter(|c| !c.location.is_on_chain()))
            .enumerate()
            .map(|(i, (data, cell))| {
                let mut output = packed::CellOutput::new_builder().lock(lock.clone());
                let mut type_id: Option<H256> = None;
                if cell.create_type_id {
                    let type_script = build_type_id_script(&inputs[0], i as u64);
                    type_id = Some(type_script.calc_script_hash().unpack());
                    output = output.type_(Some(type_script).pack());
                }
                context.cells.push((cell.to_owned(), i as u32, type_id));
                output
                    .build_exact_capacity(Capacity::bytes(data.len()).expect("bytes"))
                    .expect("build")
            })
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs.pack())
            .outputs(outputs.pack())
            .outputs_data(cell_data.pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(&tx);
        let tx = self.wallet.sign_tx(&tx);
        Some(tx)
    }

    fn build_dep_groups_deploy_tx(
        &self,
        context: &mut DeploymentContext,
        cells: HashMap<String, (H256, u32)>,
        dep_groups: &[DepGroup],
        lock: packed::Script,
    ) -> TransactionView {
        let mut cell_data: Vec<Bytes> = Vec::new();
        let mut outputs: Vec<packed::CellOutput> = Vec::new();
        let mut capacity = 0;
        for (i, dep_group) in dep_groups.iter().enumerate() {
            let out_points: packed::OutPointVec = dep_group
                .cells
                .iter()
                .map(|cell_name| {
                    let (tx_hash, index) = cells.get(cell_name).expect("get cell");
                    packed::OutPoint::new_builder()
                        .tx_hash(tx_hash.pack())
                        .index(index.pack())
                        .build()
                })
                .pack();
            let data = out_points.as_bytes();
            let data_len = data.len();
            cell_data.push(data);
            let output = packed::CellOutput::new_builder()
                .lock(lock.clone())
                .build_exact_capacity(Capacity::bytes(data_len).expect("bytes"))
                .expect("build");
            outputs.push(output);
            capacity += data_len;
            context.dep_groups.push((dep_group.to_owned(), i as u32))
        }
        let live_cells = self
            .wallet
            .find_live_cells(Capacity::bytes(capacity).expect("capacity"), self.tx_fee);
        let inputs: Vec<_> = live_cells
            .into_iter()
            .map(|cell| {
                let out_point = packed::OutPoint::new_builder()
                    .tx_hash(cell.created_by.tx_hash.pack())
                    .index((cell.created_by.index.value() as u32).pack())
                    .build();
                packed::CellInput::new_builder()
                    .previous_output(out_point)
                    .build()
            })
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs.pack())
            .outputs_data(cell_data.pack())
            .build();
        let tx = self.wallet.complete_tx_lock_deps(&tx);
        self.wallet.sign_tx(&tx)
    }
}

pub fn build_type_id_script(input: &packed::CellInput, output_index: u64) -> packed::Script {
    let mut blake2b = new_blake2b();
    blake2b.update(&input.as_slice());
    blake2b.update(&output_index.to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    let script_arg = Bytes::from(ret.to_vec());
    packed::Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(script_arg.pack())
        .build()
}
