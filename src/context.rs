use ckb_script::DataLoader;
use ckb_types::{
    bytes::Bytes,
    core::{cell::CellMeta, BlockExt, EpochExt, HeaderView},
    packed::{Byte32, CellOutput, OutPoint},
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Context {
    pub cells: HashMap<OutPoint, (CellOutput, Bytes)>,
    pub headers: HashMap<Byte32, HeaderView>,
    pub epoches: HashMap<Byte32, EpochExt>,
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
