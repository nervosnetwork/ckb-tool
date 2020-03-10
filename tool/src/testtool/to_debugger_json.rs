use super::context::Context;
use ckb_sdk_types::transaction::{
    MockCellDep, MockInfo, MockInput, MockTransaction, ReprMockTransaction,
};
use ckb_types::{core::TransactionView, packed::CellInput, prelude::*};

impl Context {
    pub fn output_debugger_json(
        &self,
        tx: TransactionView,
    ) -> Result<String, serde_json::error::Error> {
        let inputs = tx
            .input_pts_iter()
            .map(|i| {
                let (output, data) = self.cells.get(&i).expect("get cell");
                MockInput {
                    input: CellInput::new_builder().previous_output(i).build(),
                    output: output.to_owned(),
                    data: data.to_owned(),
                }
            })
            .collect();
        let cell_deps = tx
            .cell_deps_iter()
            .map(|i| {
                let (output, data) = self.cells.get(&i.out_point()).expect("get cell dep");
                MockCellDep {
                    cell_dep: i,
                    output: output.to_owned(),
                    data: data.to_owned(),
                }
            })
            .collect();
        let header_deps = tx
            .header_deps_iter()
            .map(|header_hash| {
                self.headers
                    .get(&header_hash)
                    .expect("get header")
                    .to_owned()
            })
            .collect();
        let mock_info = MockInfo {
            inputs,
            cell_deps,
            header_deps,
        };
        let mock_tx: ReprMockTransaction = MockTransaction {
            mock_info,
            tx: tx.data(),
        }
        .into();
        serde_json::to_string_pretty(&mock_tx)
    }
}
