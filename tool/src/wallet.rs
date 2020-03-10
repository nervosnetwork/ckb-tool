use super::rpc_client::RpcClient;
use ckb_crypto::secp::{Privkey, Pubkey};
use ckb_jsonrpc_types::LiveCell;
use ckb_types::{
    core::{Capacity, ScriptHashType, TransactionView},
    packed::*,
    prelude::*,
};
use std::path::PathBuf;
use std::fs;
use std::io::Read;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WalletConfig {
    pub default_code_hash: [u8; 32],
    pub rpc_uri: String,
    pub privkey_path: PathBuf,
}

pub struct Wallet {
    privkey: Privkey,
    pubkey: Pubkey,
    config: WalletConfig,
    rpc_client: Option<RpcClient>,
}

impl Wallet {
    pub fn load(config: WalletConfig) -> Self {
        let mut buf = Vec::new();
        fs::File::open(&config.privkey_path).expect("open").read_to_end(&mut buf).expect("read");
        let privkey = Privkey::from_slice(&buf);
        let pubkey = privkey.pubkey().expect("pubkey");
        Wallet{
            rpc_client: Some(RpcClient::new(&config.rpc_uri)),
            privkey,
            config,
            pubkey,
        }
    }

    pub fn rpc_client(&self) -> &RpcClient {
        self.rpc_client.as_ref().expect("rpc client")
    }

    fn collect_live_cells(&self, lock_hash: Byte32, capacity: Capacity) -> Vec<LiveCell> {
        const PER_PAGE: u64 = 20u64;

        let mut live_cells = Vec::new();
        let mut collected_capacity = 0;
        for i in 0.. {
            let cells = self.rpc_client().get_live_cells_by_lock_hash(
                lock_hash.clone(),
                i as u64,
                PER_PAGE,
                None,
            );
            if cells.is_empty() {
                panic!("can't find enough live cells");
            }
            let iter = cells.into_iter().filter(|cell| {
                cell.output_data_len.value() == 0 && cell.cell_output.type_.is_none()
            });
            for cell in iter {
                let cell_capacity = cell.cell_output.capacity.value();
                live_cells.push(cell);
                collected_capacity += cell_capacity;
                if collected_capacity > capacity.as_u64() {
                    break;
                }
            }
            if collected_capacity > capacity.as_u64() {
                break;
            }
        }
        live_cells
    }

    pub fn find_live_cells(&self, capacity: Capacity) -> Vec<LiveCell> {
        let lock_hash = self.generate_lock().calc_script_hash();
        self.collect_live_cells(lock_hash, capacity)
    }

    pub fn pubkey_hash(&self) -> [u8; 20] {
        let mut hasher = ckb_hash::new_blake2b();
        hasher.update(&self.pubkey.serialize());
        let mut hash = [0u8; 20];
        hasher.finalize(&mut hash);
        hash
    }

    pub fn generate_lock(&self) -> Script {
        let pubkey_hash = self.pubkey_hash();
        Script::new_builder()
            .code_hash(self.config.default_code_hash.pack())
            .hash_type(ScriptHashType::Data.into())
            .args(pubkey_hash.pack())
            .build()
    }

    pub fn sign_tx(&self, tx: &TransactionView) -> TransactionView {
        let hash = tx.hash().unpack();
        let sig = self
            .privkey
            .sign_recoverable(&hash)
            .expect("sign")
            .serialize();
        tx.as_advanced_builder().witness(sig.pack()).build()
    }
}
