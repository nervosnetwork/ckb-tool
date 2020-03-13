use crate::{templates, utils};
use ckb_tool::ckb_crypto::secp::Privkey;
use ckb_tool::deployment::{Deployment, DeploymentConfig};
use ckb_tool::faster_hex::hex_decode;
use ckb_tool::rpc_client::RpcClient;
use ckb_tool::wallet::{MetaData, Wallet, WalletConfig};
use clap::{App, Arg, SubCommand};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::exit;

pub fn run_cli() {
    let matches = App::new("ckb tool")
        .version("1.0")
        .about("CKB tools")
        .subcommand(
            SubCommand::with_name("key-gen")
                .about("Generate privkey and write it to a file")
                .arg(
                    Arg::with_name("file")
                        .long("file")
                        .short("f")
                        .help("The output location of privkey")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("print-single-lock")
                .about("Print default single lock script")
                .arg(
                    Arg::with_name("uri")
                        .long("uri")
                        .short("u")
                        .takes_value(true)
                        .default_value("http://localhost:8114")
                        .help("Node RPC uri"),
                )
                .arg(
                    Arg::with_name("pubkey_hash")
                        .long("pubkey")
                        .short("p")
                        .takes_value(true)
                        .help("Pubkey hash in hex"),
                ),
        )
        .subcommand(
            SubCommand::with_name("init-deploy")
                .about("Generate deployment template in current directory")
                .arg(
                    Arg::with_name("dir")
                        .long("dir")
                        .short("d")
                        .takes_value(true)
                        .help("Init in directory"),
                ),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deployment cells and dep groups")
                .arg(
                    Arg::with_name("wallet")
                        .long("wallet")
                        .short("w")
                        .help("Wallet configure file")
                        .default_value("wallet.toml")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("deployment")
                        .long("deployment")
                        .short("d")
                        .help("Deployment configure file")
                        .default_value("deployment.toml")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("reindex")
                .about("Reindex live cells")
                .arg(
                    Arg::with_name("wallet")
                        .long("wallet")
                        .short("w")
                        .help("Wallet configure file")
                        .default_value("wallet.toml")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("index-state")
                .about("Query index state")
                .arg(
                    Arg::with_name("wallet")
                        .long("wallet")
                        .short("w")
                        .help("Wallet configure file")
                        .default_value("wallet.toml")
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("key-gen", Some(sub_m)) => {
            let file = sub_m.value_of("file").expect("file");
            let seed = utils::random_privkey_seed();
            fs::File::create(&file)
                .expect("open")
                .write_all(&seed)
                .expect("write");
            let privkey = Privkey::from_slice(&seed);
            let pubkey_hash = utils::pubkey_hash(&privkey.pubkey().expect("pubkey"));
            println!("pubkey hash: {:#x}", pubkey_hash);
            println!("privkey path: {}", file);
        }
        ("print-single-lock", Some(sub_m)) => {
            let uri = sub_m.value_of("uri").expect("uri");
            let pubkey_hash = sub_m.value_of("pubkey").expect("pubkey hash");
            let pubkey_bytes = pubkey_hash.as_bytes();
            let mut buf = vec![0; pubkey_bytes.len() >> 1];
            hex_decode(pubkey_bytes, &mut buf).expect("dehex");
            let lock_script = MetaData::load(&RpcClient::new(uri))
                .build_default_single_lock()
                .build_script(buf.into());
            println!("lock hash: {:#x}", lock_script.calc_script_hash());
            println!("lock: {}", lock_script);
        }
        ("init-deploy", Some(sub_m)) => {
            let dir: PathBuf = sub_m.value_of("dir").unwrap_or_default().into();
            for (name, template) in &[
                ("wallet.toml", templates::WALLET_TEMPLATE.clone()),
                ("deployment.toml", templates::DEPLOYMENT_TEMPLATE.clone()),
            ] {
                let path = dir.join(name);
                println!("copy to {:?}", &path);
                fs::File::create(path)
                    .expect("open")
                    .write_all(&template)
                    .expect("failed write template");
            }
            println!("done");
        }
        ("index-state", Some(sub_m)) => {
            let wallet_path = sub_m.value_of("wallet").expect("wallet");

            let mut buf = Vec::new();
            buf.clear();
            fs::File::open(wallet_path)
                .expect("open")
                .read_to_end(&mut buf)
                .expect("read");
            let wallet_config: WalletConfig = toml::from_slice(&buf).expect("parse toml");
            let wallet = Wallet::load(wallet_config);
            match wallet.get_index_state() {
                Some(index_state) => {
                    println!("lock hash:");
                    println!("{:#x}", index_state.lock_hash);
                    println!("");
                    println!("indexed block:");
                    println!("number: {}", index_state.block_number.value());
                    println!("hash: {:#x}", index_state.block_hash);
                }
                None => {
                    println!("index state not found");
                }
            }
        }
        ("reindex", Some(sub_m)) => {
            let wallet_path = sub_m.value_of("wallet").expect("wallet");

            let mut buf = Vec::new();
            buf.clear();
            fs::File::open(wallet_path)
                .expect("open")
                .read_to_end(&mut buf)
                .expect("read");
            let wallet_config: WalletConfig = toml::from_slice(&buf).expect("parse toml");
            let wallet = Wallet::load(wallet_config);
            wallet.reindex();
            println!("reindexing..");
        }
        ("deploy", Some(sub_m)) => {
            let deployment_path = sub_m.value_of("deployment").expect("deployment");
            let wallet_path = sub_m.value_of("wallet").expect("wallet");

            let mut buf = Vec::new();
            fs::File::open(deployment_path)
                .expect("open")
                .read_to_end(&mut buf)
                .expect("read");
            let deployment_config: DeploymentConfig = toml::from_slice(&buf).expect("parse toml");
            buf.clear();
            fs::File::open(wallet_path)
                .expect("open")
                .read_to_end(&mut buf)
                .expect("read");
            let wallet_config: WalletConfig = toml::from_slice(&buf).expect("parse toml");

            let wallet = Wallet::load(wallet_config);
            println!("wallet pubkey hash: {:#x}", wallet.pubkey_hash());
            let mut deployment = Deployment::new(wallet);
            println!("start deployment process");
            println!("...");
            let context = deployment.process(deployment_config);
            println!("done");
            println!("report:");
            println!("");
            println!("cells");
            println!("===============");
            println!("tx_hash {:#x}", context.cells_deploy_tx_hash);
            for (cell, index, type_id) in &context.cells {
                let type_id = type_id.to_owned().unwrap_or_default();
                println!("-> {}: index {} type_id {:#x}", cell.name, index, type_id);
            }
            println!();
            println!("dep groups");
            println!("===============");
            println!("tx_hash {:#x}", context.dep_groups_deploy_tx_hash);
            for (dep_group, index) in &context.dep_groups {
                println!("-> {}: index {}", dep_group.name, index);
            }
        }
        (name, _sub_m) => {
            eprintln!("Unknown subcommand {}", name);
            exit(1);
        }
    }
}
