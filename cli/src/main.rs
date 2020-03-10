use ckb_tool::deployment::{Deployment, DeploymentConfig};
use ckb_tool::wallet::{Wallet, WalletConfig};
use std::fs;
use std::io::Read;

fn main() {
    let mut buf = Vec::new();
    fs::File::open("deployment.toml")
        .expect("open")
        .read_to_end(&mut buf)
        .expect("read");
    let deployment_config: DeploymentConfig = toml::from_slice(&buf).expect("parse toml");
    fs::File::open("wallet.toml")
        .expect("open")
        .read_to_end(&mut buf)
        .expect("read");
    let wallet_config: WalletConfig = toml::from_slice(&buf).expect("parse toml");

    let wallet = Wallet::load(wallet_config);
    Deployment::new(wallet).process(deployment_config);
    println!("done");
}
