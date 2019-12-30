# ckb-contract-tool

Collect some helper methods for CKB contract testing.

## Usage

``` rust
let mut binary = Vec::new();
File::open("my-contract-binary").unwrap().read_to_end(&mut binary).expect("read contract binary");
let binary = Bytes::from(binary);
let mut context = Context::default();
context.deploy_contract(binary.clone());
let tx = TxBuilder::default().lock_bin(binary).inject_and_build(&mut context).expect("build tx");
let verify_result = context.verify_tx(&tx, 500000u64);
verify_result.expect("pass verification");
```
