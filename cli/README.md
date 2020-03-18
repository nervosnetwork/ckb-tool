# ckb-tool-cli

NOTICE, please only use `ckb-tool-cli` in the testnet or local dev chain. This tool is not production-ready yet, use it in the mainnet may cause money lost.

## Usage

#### Deploy a contract

``` sh
mkdir deploy && cd deploy

# init deployment template
ckb-tool-cli init-deploy

# modify wallet.toml and deployment.toml

# start deployment
ckb-tool-cli deploy

# if deployment success, you will see output:
# cells
# ===============
# tx_hash 0xd2152dafa66a478bb67bbebd5aaa298cca0f6ada51d0559e0edbd37ec46ed1c7
# -> udt_wallet: index 0 type_id 0x2c7952dced34253e620251f3d8f6ef11749176329f51d6fe9a6d8f7b67f04622
# 
# dep groups
# ===============
# tx_hash 0xbbf893630ba4192b96943bf8193aa19f23f72ecbea4f9268e8adcff3ff053eea
# -> udt_wallet: index 0
```

## LICENSE

MIT
