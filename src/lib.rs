mod context;
mod to_debugger_json;
mod tx_builder;

pub use context::Context;
pub use tx_builder::TxBuilder;

// re-exports
pub use ckb_error;
pub use ckb_hash;
pub use ckb_script::{self, DataLoader};
pub use ckb_types;
pub use ckb_types::bytes;

pub fn calc_data_hash(data: &[u8]) -> ckb_types::packed::Byte32 {
    use ckb_types::packed::CellOutput;
    CellOutput::calc_data_hash(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ckb_types::{core::ScriptHashType, packed::*, prelude::*};
    #[test]
    fn test_dummy_lock() {
        let mut context = Context::default();
        let lock_bin = bytes::Bytes::new();
        context.deploy_contract(lock_bin.clone());
        let lock_code_hash = calc_data_hash(&lock_bin);
        let tx = TxBuilder::default()
            .lock_script(
                Script::new_builder()
                    .code_hash(lock_code_hash)
                    .hash_type(ScriptHashType::Data.into())
                    .build()
                    .as_bytes(),
            )
            .inject_and_build(&mut context)
            .expect("build tx");
        let verify_result = context.verify_tx(&tx, std::u32::MAX.into());
        assert!(verify_result.is_err());
    }
}
