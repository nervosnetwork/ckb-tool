use ckb_tool::ckb_crypto::secp::Privkey;
use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::H160;
use ckb_tool::deployment::{Deployment, DeploymentConfig};
use ckb_tool::wallet::{Wallet, WalletConfig};
use rand::{thread_rng, Rng};
use std::fs;
use std::io::{Read, Write};

fn main() {
    let mut rng = thread_rng();
    let mut seed = vec![0; 32];
    rng.fill(seed.as_mut_slice());
    fs::File::create("privkey")
        .expect("open")
        .write_all(&seed)
        .expect("write");
    let privkey = Privkey::from_slice(&seed);
    let mut hasher = new_blake2b();
    hasher.update(&privkey.pubkey().expect("pubkey").serialize());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    let mut hash160 = [0u8; 20];
    hash160.copy_from_slice(&hash[..20]);
    let pubkey_hash: H160 = hash160.into();
    println!("pubkey hash: {:#x}", pubkey_hash);

    let mut buf = Vec::new();
    fs::File::open("deployment.toml")
        .expect("open")
        .read_to_end(&mut buf)
        .expect("read");
    let deployment_config: DeploymentConfig = toml::from_slice(&buf).expect("parse toml");
    buf.clear();
    fs::File::open("wallet.toml")
        .expect("open")
        .read_to_end(&mut buf)
        .expect("read");
    let wallet_config: WalletConfig = toml::from_slice(&buf).expect("parse toml");

    let wallet = Wallet::load(wallet_config);
    let deployment = Deployment::new(wallet);
    let context = deployment.process(deployment_config);
    println!("done");
    println!("deploy cells...");
    println!("===============");
    println!("tx_hash {:#x}", context.cells_deploy_tx_hash);
    for (cell, index, type_id) in &context.cells {
        let type_id = type_id.to_owned().unwrap_or_default();
        println!("-> {}: index {} type_id {:#x}", cell.name, index, type_id);
    }
    println!();
    println!("deploy dep groups...");
    println!("===============");
    println!("tx_hash {:#x}", context.dep_groups_deploy_tx_hash);
    for (dep_group, index) in &context.dep_groups {
        println!("-> {}: index {}", dep_group.name, index);
    }
}
