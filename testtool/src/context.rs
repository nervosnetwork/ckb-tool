use crate::tx_verifier::OutputsDataVerifier;
use ckb_tool::ckb_error::Error as CKBError;
use ckb_tool::ckb_script::{DataLoader, TransactionScriptsVerifier};
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{
        cell::{CellMeta, CellMetaBuilder, ResolvedTransaction},
        BlockExt, Capacity, Cycle, DepType, EpochExt, HeaderView, ScriptHashType, TransactionView,
    },
    packed::{Byte32, CellDep, CellOutput, OutPoint, Script},
    prelude::*,
};
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};

pub fn random_hash() -> Byte32 {
    let mut rng = thread_rng();
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.pack()
}
pub fn random_out_point() -> OutPoint {
    OutPoint::new_builder().tx_hash(random_hash()).build()
}

#[derive(Default)]
pub struct Context {
    pub cells: HashMap<OutPoint, (CellOutput, Bytes)>,
    pub headers: HashMap<Byte32, HeaderView>,
    pub epoches: HashMap<Byte32, EpochExt>,
    pub contracts: HashMap<Byte32, OutPoint>,
}

impl Context {
    pub fn deploy_contract(&mut self, data: Bytes) -> OutPoint {
        let data_hash = CellOutput::calc_data_hash(&data);
        if let Some(out_point) = self.contracts.get(&data_hash) {
            // contract has been deployed
            return out_point.to_owned();
        }
        let mut rng = thread_rng();
        let tx_hash = {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf);
            buf.pack()
        };
        let out_point = OutPoint::new(tx_hash.clone(), 0);
        let cell = CellOutput::new_builder()
            .capacity(Capacity::bytes(data.len()).expect("script capacity").pack())
            .build();
        self.cells.insert(out_point.clone(), (cell, data.into()));
        self.contracts.insert(data_hash, out_point.clone());
        out_point
    }

    pub fn get_contract_out_point(&self, data_hash: &Byte32) -> Option<OutPoint> {
        self.contracts.get(data_hash).cloned()
    }

    pub fn insert_cell(&mut self, out_point: OutPoint, cell: CellOutput, data: Bytes) {
        self.cells.insert(out_point, (cell, data));
    }

    pub fn get_cell(&self, out_point: &OutPoint) -> Option<(CellOutput, Bytes)> {
        self.cells.get(out_point).cloned()
    }

    /// build script with out_point
    pub fn build_script(&mut self, out_point: &OutPoint, args: Bytes) -> Option<Script> {
        let (_, contract_data) = self.cells.get(out_point)?;
        let data_hash = CellOutput::calc_data_hash(contract_data);
        Some(
            Script::new_builder()
                .code_hash(data_hash)
                .hash_type(ScriptHashType::Data.into())
                .args(args.pack())
                .build(),
        )
    }

    /// create a cell with random out_point
    pub fn create_cell(&mut self, cell: CellOutput, data: Bytes) -> OutPoint {
        let out_point = random_out_point();
        self.cells.insert(out_point.clone(), (cell, data));
        out_point
    }

    fn find_cell_dep_for_script(&self, script: &Script) -> CellDep {
        if script.hash_type() != ScriptHashType::Data.into() {
            panic!("do not support hash_type {} yet", script.hash_type());
        }

        let out_point = self
            .get_contract_out_point(&script.code_hash())
            .expect("find contract out point");
        CellDep::new_builder()
            .out_point(out_point)
            .dep_type(DepType::Code.into())
            .build()
    }

    /// complete cell deps for a transaction
    pub fn complete_tx(&mut self, tx: TransactionView) -> TransactionView {
        let mut cell_deps: HashSet<CellDep> = HashSet::new();
        for i in tx.input_pts_iter() {
            if let Some((cell, _data)) = self.cells.get(&i) {
                let dep = self.find_cell_dep_for_script(&cell.lock());
                cell_deps.insert(dep);

                if let Some(script) = cell.type_().to_opt() {
                    let dep = self.find_cell_dep_for_script(&script);
                    cell_deps.insert(dep);
                }
            }
        }

        for (cell, _data) in tx.outputs_with_data_iter() {
            if let Some(script) = cell.type_().to_opt() {
                let dep = self.find_cell_dep_for_script(&script);
                cell_deps.insert(dep);
            }
        }

        for cell_dep in tx.cell_deps_iter() {
            cell_deps.insert(cell_dep);
        }

        tx.as_advanced_builder()
            .set_cell_deps(Vec::new())
            .cell_deps(cell_deps.into_iter().collect::<Vec<_>>().pack())
            .build()
    }

    fn build_resolved_tx(&self, tx: &TransactionView) -> ResolvedTransaction {
        let input_cells = tx
            .inputs()
            .into_iter()
            .map(|input| {
                let previous_out_point = input.previous_output();
                let (input_output, input_data) = self.cells.get(&previous_out_point).unwrap();
                CellMetaBuilder::from_cell_output(
                    input_output.to_owned(),
                    input_data.to_vec().into(),
                )
                .out_point(previous_out_point)
                .build()
            })
            .collect();
        let resolved_cell_deps = tx
            .cell_deps()
            .into_iter()
            .map(|deps_out_point| {
                let (dep_output, dep_data) = self.cells.get(&deps_out_point.out_point()).unwrap();
                CellMetaBuilder::from_cell_output(dep_output.to_owned(), dep_data.to_vec().into())
                    .out_point(deps_out_point.out_point())
                    .build()
            })
            .collect();
        ResolvedTransaction {
            transaction: tx.clone(),
            resolved_cell_deps,
            resolved_inputs: input_cells,
            resolved_dep_groups: vec![],
        }
    }

    // check format and consensus rules
    fn verify_tx_consensus(&self, tx: &TransactionView) -> Result<(), CKBError> {
        OutputsDataVerifier::new(tx).verify()?;
        Ok(())
    }

    pub fn verify_tx(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, CKBError> {
        self.verify_tx_consensus(tx)?;
        let resolved_tx = self.build_resolved_tx(tx);
        let mut verifier = TransactionScriptsVerifier::new(&resolved_tx, self);
        verifier.set_debug_printer(|_id, msg| {
            println!("[contract debug] {}", msg);
        });
        verifier.verify(max_cycles)
    }
}

impl DataLoader for Context {
    // load Cell Data
    fn load_cell_data(&self, cell: &CellMeta) -> Option<(Bytes, Byte32)> {
        cell.mem_cell_data
            .as_ref()
            .map(|(data, hash)| (Bytes::from(data.to_vec()), hash.to_owned()))
            .or_else(|| {
                self.cells.get(&cell.out_point).map(|(_, data)| {
                    (
                        Bytes::from(data.to_vec()),
                        CellOutput::calc_data_hash(&data),
                    )
                })
            })
    }
    // load BlockExt
    fn get_block_ext(&self, _hash: &Byte32) -> Option<BlockExt> {
        unreachable!()
    }

    // load header
    fn get_header(&self, block_hash: &Byte32) -> Option<HeaderView> {
        self.headers.get(block_hash).cloned()
    }

    // load EpochExt
    fn get_block_epoch(&self, block_hash: &Byte32) -> Option<EpochExt> {
        self.epoches.get(block_hash).cloned()
    }
}
