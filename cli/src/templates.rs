use ckb_tool::bytes::Bytes;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref WALLET_TEMPLATE: Bytes = Bytes::from(include_bytes!("../templates/wallet.toml").to_vec());
    pub static ref DEPLOYMENT_TEMPLATE: Bytes = Bytes::from(include_bytes!("../templates/deployment.toml").to_vec());
}
