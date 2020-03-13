use super::rpc_client::RpcClient;
use ckb_crypto::secp::{Privkey, Pubkey};
use ckb_hash::new_blake2b;
use ckb_jsonrpc_types::{CellDep, LiveCell, LockHashIndexState, ScriptHashType};
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, DepType, ScriptHashType as CoreScriptHashType, TransactionView},
    packed,
    prelude::*,
    H160, H256,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Lock {
    pub code_hash: H256,
    pub hash_type: ScriptHashType,
    pub cell_deps: Vec<CellDep>,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WalletConfig {
    pub rpc_uri: String,
    pub privkey_path: PathBuf,
    #[serde(default)]
    pub lock: Lock,
}

/// chain metadata
#[derive(Default)]
pub struct MetaData {
    pub genesis_cellbase_tx_hash: H256,
    pub genesis_dep_group_tx_hash: H256,
    pub secp256k1_lock_type_id: H256,
}

pub struct Wallet {
    privkey: Privkey,
    pubkey: Pubkey,
    config: WalletConfig,
    metadata: MetaData,
    rpc_client: Option<RpcClient>,
    used_cells: HashSet<LiveCellKey>,
}

impl Wallet {
    pub fn load(config: WalletConfig) -> Self {
        let mut buf = Vec::new();
        fs::File::open(&config.privkey_path)
            .expect("open")
            .read_to_end(&mut buf)
            .expect("read");
        let privkey = Privkey::from_slice(&buf);
        let pubkey = privkey.pubkey().expect("pubkey");
        let mut wallet = Wallet {
            rpc_client: Some(RpcClient::new(&config.rpc_uri)),
            privkey,
            config,
            pubkey,
            metadata: Default::default(),
            used_cells: Default::default(),
        };
        wallet.init_data();
        wallet
    }

    pub fn rpc_client(&self) -> &RpcClient {
        self.rpc_client.as_ref().expect("rpc client")
    }

    fn collect_live_cells(&self, lock_hash: packed::Byte32, capacity: Capacity) -> Vec<LiveCell> {
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
                // cell is in use, but not yet committed
                if self.used_cells.contains(&live_cell_key(&cell)) {
                    continue;
                }
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

    pub fn find_live_cells(&self, capacity: Capacity, fee: Capacity) -> Vec<LiveCell> {
        let lock_hash = self.default_lock().calc_script_hash();
        self.collect_live_cells(lock_hash, capacity.safe_add(fee).expect("overflow"))
    }

    /// send transaction
    pub fn send_transaction(&mut self, tx: &TransactionView) {
        self.rpc_client().send_transaction(tx.data().into());
        for out_point in tx.input_pts_iter() {
            self.used_cells
                .insert((out_point.tx_hash().unpack(), out_point.index().unpack()));
        }
    }

    pub fn pubkey_hash(&self) -> H160 {
        let mut hasher = new_blake2b();
        hasher.update(&self.pubkey.serialize());
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(&hash[..20]);
        hash160.into()
    }

    pub fn default_lock(&self) -> packed::Script {
        let pubkey_hash = self.pubkey_hash();
        let pubkey_hash: [u8; 20] = pubkey_hash.into();
        self.config.lock.build_script(pubkey_hash.to_vec().into())
    }

    pub fn complete_tx_lock_deps(&self, tx: &TransactionView) -> TransactionView {
        let cell_deps: Vec<packed::CellDep> = self
            .config
            .lock
            .cell_deps
            .iter()
            .map(|cell_dep| cell_dep.to_owned().into())
            .collect();
        tx.as_advanced_builder().cell_deps(cell_deps.pack()).build()
    }

    pub fn sign_tx(&self, tx: &TransactionView) -> TransactionView {
        // reserve lock signature
        let witness_args = tx
            .witnesses()
            .get(0)
            .map(|data| packed::WitnessArgs::new_unchecked(data.unpack()))
            .unwrap_or_default();
        let zero_lock = [0u8; 65];
        let witness_args = witness_args
            .as_builder()
            .lock(Some(Bytes::from(zero_lock.to_vec())).pack())
            .build();
        let mut witnesses: Vec<Bytes> = tx.witnesses().unpack();
        if witnesses.is_empty() {
            witnesses.push(witness_args.as_bytes());
        } else {
            witnesses[0] = witness_args.as_bytes();
        }
        let tx = tx
            .as_advanced_builder()
            .set_witnesses(
                witnesses
                    .iter()
                    .map(|witness| witness.to_owned().pack())
                    .collect::<Vec<packed::Bytes>>(),
            )
            .build();

        // start calculate message
        let mut hasher = new_blake2b();
        hasher.update(&tx.hash().raw_data());

        for witness in &witnesses {
            hasher.update(&(witness.len() as u64).to_le_bytes());
            hasher.update(witness);
        }

        let mut message = [0u8; 32];
        hasher.finalize(&mut message);
        let message: H256 = message.into();

        let sig = self
            .privkey
            .sign_recoverable(&message)
            .expect("sign")
            .serialize();
        let lock_args: Bytes = sig.into();
        witnesses[0] = packed::WitnessArgs::new_builder()
            .lock(Some(lock_args).pack())
            .build()
            .as_bytes();
        tx.as_advanced_builder()
            .set_witnesses(
                witnesses
                    .into_iter()
                    .map(|witness| witness.pack())
                    .collect::<Vec<packed::Bytes>>(),
            )
            .build()
    }

    pub fn metadata(&self) -> &MetaData {
        &self.metadata
    }

    pub fn get_index_state(&self) -> Option<LockHashIndexState> {
        let lock = self.default_lock();
        let lock_hash: H256 = lock.calc_script_hash().unpack();
        let index_states = self.rpc_client().get_lock_hash_index_states();
        index_states
            .into_iter()
            .filter(|index| index.lock_hash == lock_hash)
            .next()
    }

    pub fn reindex(&self) {
        let lock = self.default_lock();
        let lock_hash = lock.calc_script_hash();
        self.rpc_client().deindex_lock_hash(lock_hash.clone());
        self.rpc_client().index_lock_hash(lock_hash, Some(0));
    }

    fn init_data(&mut self) {
        let metadata = MetaData::load(self.rpc_client());

        // set default lock script
        if self.config.lock == Default::default() {
            self.config.lock = metadata.build_default_single_lock();
        }

        self.metadata = metadata;

        // start index default lock
        // self.rpc_client()
        //     .index_lock_hash(self.default_lock().calc_script_hash(), Some(0));
    }
}

impl MetaData {
    pub fn load(rpc_client: &RpcClient) -> Self {
        let genesis_block = rpc_client
            .get_block_by_number(0u64.into())
            .expect("get genesis");
        let genesis_cellbase_tx = &genesis_block.transactions[0];
        let genesis_dep_group_tx = &genesis_block.transactions[1];
        let secp256k1_lock_type_id: H256 = packed::Script::from(
            genesis_cellbase_tx.inner.outputs[1]
                .type_
                .clone()
                .expect("type id"),
        )
        .calc_script_hash()
        .unpack();

        MetaData {
            genesis_cellbase_tx_hash: genesis_cellbase_tx.hash.clone(),
            genesis_dep_group_tx_hash: genesis_dep_group_tx.hash.clone(),
            secp256k1_lock_type_id,
        }
    }

    pub fn build_default_single_lock(&self) -> Lock {
        let cell_dep = packed::CellDep::new_builder()
            .dep_type(DepType::DepGroup.into())
            .out_point(
                packed::OutPoint::new_builder()
                    .tx_hash(self.genesis_dep_group_tx_hash.pack())
                    .index(0u32.pack())
                    .build(),
            )
            .build();
        Lock {
            code_hash: self.secp256k1_lock_type_id.clone(),
            hash_type: CoreScriptHashType::Type.into(),
            cell_deps: vec![cell_dep.into()],
        }
    }
}

impl Lock {
    pub fn build_script(&self, args: Bytes) -> packed::Script {
        packed::Script::new_builder()
            .code_hash(self.code_hash.pack())
            .hash_type(CoreScriptHashType::from(self.hash_type.clone()).into())
            .args(args.pack())
            .build()
    }
}

type LiveCellKey = (H256, u32);

fn live_cell_key(cell: &LiveCell) -> LiveCellKey {
    (
        cell.created_by.tx_hash.to_owned(),
        cell.created_by.index.value() as u32,
    )
}
