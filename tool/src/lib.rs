pub mod deployment;
pub mod rpc_client;
pub mod testtool;
pub mod wallet;

// re-exports
pub use ckb_error;
pub use ckb_hash;
pub use ckb_script::{self, DataLoader};
pub use ckb_types;
pub use ckb_types::bytes;
pub use ckb_crypto;
pub use faster_hex;

pub fn calc_data_hash(data: &[u8]) -> ckb_types::packed::Byte32 {
    use ckb_types::packed::CellOutput;
    CellOutput::calc_data_hash(data)
}
