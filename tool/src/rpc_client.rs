use ckb_jsonrpc_types::{
    Alert, BannedAddr, Block, BlockNumber, BlockReward, BlockTemplate, BlockView, Capacity,
    CellOutputWithOutPoint, CellTransaction, CellWithStatus, ChainInfo, Cycle, DryRunResult,
    EpochNumber, EpochView, EstimateResult, HeaderView, LiveCell, LockHashIndexState, Node,
    OutPoint, PeerState, Timestamp, Transaction, TransactionWithStatus, TxPoolInfo, Uint64,
    Version,
};
use ckb_types::core::BlockNumber as CoreBlockNumber;
use ckb_types::{packed::Byte32, prelude::*, H256};
use simple_jsonrpc_client::*;

jsonrpc!(pub struct Rpc {
    pub fn get_block(&self, _hash: H256) -> Option<BlockView>;
    pub fn get_fork_block(&self, _hash: H256) -> Option<BlockView>;
    pub fn get_block_by_number(&self, _number: BlockNumber) -> Option<BlockView>;
    pub fn get_header(&self, _hash: H256) -> Option<HeaderView>;
    pub fn get_header_by_number(&self, _number: BlockNumber) -> Option<HeaderView>;
    pub fn get_transaction(&self, _hash: H256) -> Option<TransactionWithStatus>;
    pub fn get_block_hash(&self, _number: BlockNumber) -> Option<H256>;
    pub fn get_tip_header(&self) -> HeaderView;
    pub fn get_cells_by_lock_hash(
        &self,
        _lock_hash: H256,
        _from: BlockNumber,
        _to: BlockNumber
    ) -> Vec<CellOutputWithOutPoint>;
    pub fn get_live_cell(&self, _out_point: OutPoint, _with_data: bool) -> CellWithStatus;
    pub fn get_tip_block_number(&self) -> BlockNumber;
    pub fn get_current_epoch(&self) -> EpochView;
    pub fn get_epoch_by_number(&self, number: EpochNumber) -> Option<EpochView>;

    pub fn local_node_info(&self) -> Node;
    pub fn get_peers(&self) -> Vec<Node>;
    pub fn get_banned_addresses(&self) -> Vec<BannedAddr>;
    pub fn set_ban(
        &self,
        address: String,
        command: String,
        ban_time: Option<Timestamp>,
        absolute: Option<bool>,
        reason: Option<String>
    ) -> ();

    pub fn get_block_template(
        &self,
        bytes_limit: Option<Uint64>,
        proposals_limit: Option<Uint64>,
        max_version: Option<Version>
    ) -> BlockTemplate;
    pub fn submit_block(&self, _work_id: String, _data: Block) -> H256;
    pub fn get_blockchain_info(&self) -> ChainInfo;
    pub fn get_peers_state(&self) -> Vec<PeerState>;
    pub fn compute_transaction_hash(&self, tx: Transaction) -> H256;
    pub fn dry_run_transaction(&self, _tx: Transaction) -> DryRunResult;
    pub fn send_transaction(&self, tx: Transaction) -> H256;
    pub fn tx_pool_info(&self) -> TxPoolInfo;

    pub fn send_alert(&self, alert: Alert) -> ();

    pub fn add_node(&self, peer_id: String, address: String) -> ();
    pub fn remove_node(&self, peer_id: String) -> ();
    pub fn process_block_without_verify(&self, _data: Block) -> Option<H256>;

    pub fn get_live_cells_by_lock_hash(&self, lock_hash: H256, page: Uint64, per_page: Uint64, reverse_order: Option<bool>) -> Vec<LiveCell>;
    pub fn get_transactions_by_lock_hash(&self, lock_hash: H256, page: Uint64, per_page: Uint64, reverse_order: Option<bool>) -> Vec<CellTransaction>;
    pub fn index_lock_hash(&self, lock_hash: H256, index_from: Option<BlockNumber>) -> LockHashIndexState;
    pub fn deindex_lock_hash(&self, lock_hash: H256) -> ();
    pub fn get_lock_hash_index_states(&self) -> Vec<LockHashIndexState>;
    pub fn calculate_dao_maximum_withdraw(&self, _out_point: OutPoint, _hash: H256) -> Capacity;
    pub fn get_cellbase_output_capacity_details(&self, _hash: H256) -> Option<BlockReward>;
    pub fn broadcast_transaction(&self, tx: Transaction, cycles: Cycle) -> H256;
    pub fn estimate_fee_rate(&self, expect_confirm_blocks: Uint64) -> EstimateResult;
});

pub struct RpcClient {
    rpc: Rpc,
}

impl RpcClient {
    pub fn new(uri: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .gzip(true)
            .timeout(::std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            rpc: Rpc::new(uri, client),
        }
    }

    pub fn inner(&self) -> &Rpc {
        &self.rpc
    }

    pub fn get_blockchain_info(&self) -> ChainInfo {
        self.inner()
            .get_blockchain_info()
            .expect("rpc call get_blockchain_info")
    }

    pub fn send_transaction(&self, tx: Transaction) -> Byte32 {
        self.inner()
            .send_transaction(tx)
            .expect("rpc call send_transaction")
            .pack()
    }

    pub fn get_transaction(&self, tx_hash: H256) -> Option<TransactionWithStatus> {
        self.inner()
            .get_transaction(tx_hash)
            .expect("rpc call get_transaction")
    }

    pub fn get_live_cells_by_lock_hash(
        &self,
        lock_hash: Byte32,
        page: u64,
        per_page: u64,
        reverse_order: Option<bool>,
    ) -> Vec<LiveCell> {
        self.inner()
            .get_live_cells_by_lock_hash(
                lock_hash.unpack(),
                page.into(),
                per_page.into(),
                reverse_order,
            )
            .expect("rpc call get_live_cells_by_lock_hash")
    }

    pub fn get_transactions_by_lock_hash(
        &self,
        lock_hash: Byte32,
        page: u64,
        per_page: u64,
        reverse_order: Option<bool>,
    ) -> Vec<CellTransaction> {
        self.inner()
            .get_transactions_by_lock_hash(
                lock_hash.unpack(),
                page.into(),
                per_page.into(),
                reverse_order,
            )
            .expect("rpc call get_transactions_by_lock_hash")
    }

    pub fn index_lock_hash(
        &self,
        lock_hash: Byte32,
        index_from: Option<CoreBlockNumber>,
    ) -> LockHashIndexState {
        self.inner()
            .index_lock_hash(lock_hash.unpack(), index_from.map(Into::into))
            .expect("rpc call index_lock_hash")
    }

    pub fn deindex_lock_hash(&self, lock_hash: Byte32) {
        self.inner()
            .deindex_lock_hash(lock_hash.unpack())
            .expect("rpc call deindex_lock_hash")
    }

    pub fn get_lock_hash_index_states(&self) -> Vec<LockHashIndexState> {
        self.inner()
            .get_lock_hash_index_states()
            .expect("rpc call get_lock_hash_index_states")
    }

    pub fn get_block_by_number(&self, n: BlockNumber) -> Option<BlockView> {
        self.inner()
            .get_block_by_number(n)
            .expect("rpc call get_block_by_number")
    }
}
