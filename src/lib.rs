mod context;
mod tx_builder;

pub use context::Context;
pub use tx_builder::TxBuilder;

// re-exports
pub use ckb_script::DataLoader;
pub use ckb_types;
pub use ckb_error;

const MAX_CYCLES: u64 = std::u32::MAX as u64;

#[test]
fn test_dummy_lock() {
    let verify_result = TxBuilder::default().lock_bin(Vec::new()).verify();
    assert!(verify_result.is_err());
}
