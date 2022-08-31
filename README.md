# cw20-bid
A smart contract written in Rust and built with CosmWasm to simulate an auction process that allows buyers to place bids and settle the payment.

### Version
- [Rust](https://www.rust-lang.org/): 1.61.0
- [CosmWasm](https://cosmwasm.com/): 1.0.0
- [wasmd](https://github.com/CosmWasm/wasmd): 0.27.0
- [cw20-base](https://github.com/CosmWasm/cw-plus): 0.13.4

### Installation
Install Rust.
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build
```
cargo build
```

### Compile
```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.6
```

### Schema
Generate JSON schema files. The files will be saved to the `schema` folder.
```
cargo schema
```

### Get Started
1. Set up parameters.
```
export CHAIN_ID="malaga-420"
export RPC="https://rpc.malaga-420.cosmwasm.com:443"
export FAUCET="https://faucet.malaga-420.cosmwasm.com"
export NODE=(--node $RPC)
export TXFLAG=($NODE --chain-id $CHAIN_ID --gas-prices 0.25umlg --gas auto --gas-adjustment 1.3)
```

2. Create wallets.
```
wasmd keys add wallet1
wasmd keys add wallet2
wasmd keys add wallet3
WALLET1=$(wasmd keys show -a wallet1)
WALLET2=$(wasmd keys show -a wallet2)
WALLET3=$(wasmd keys show -a wallet3)
```

3. Request tokens from faucet.
```
JSON=$(jq -n --arg addr $WALLET1 '{"denom":"umlg","address":$addr}') && curl -X POST --header "Content-Type: application/json" --data "$JSON" "$FAUCET"/credit
JSON=$(jq -n --arg addr $WALLET2 '{"denom":"umlg","address":$addr}') && curl -X POST --header "Content-Type: application/json" --data "$JSON" "$FAUCET"/credit
JSON=$(jq -n --arg addr $WALLET3 '{"denom":"umlg","address":$addr}') && curl -X POST --header "Content-Type: application/json" --data "$JSON" "$FAUCET"/credit
```

4. Download `cw20_base.wasm` and upload it to the chain.
```
wget https://github.com/CosmWasm/cw-plus/releases/download/v0.13.4/cw20_base.wasm
RES1=$(wasmd tx wasm store cw20_base.wasm --from wallet1 $TXFLAG -y --output json -b block)
```

5. Get the code ID of the uploaded binary.
```
CODE_ID1=$(echo $RES1 | jq -r '.logs[0].events[-1].attributes[0].value')
```

6. Create a custom CW20 token.
```
INIT='{"name":"Moo","symbol":"MOO","decimals":6,"initial_balances":[{"address":"'$WALLET1'","amount":"10000000000"},{"address":"'$WALLET2'","amount":"10000000000"},{"address":"'$WALLET3'","amount":"10000000000"}],"mint":{"minter":"'$WALLET1'"}}'
wasmd tx wasm instantiate $CODE_ID1 "$INIT" \
    --from wallet1 --label "cw20 base" \
    $TXFLAG -y --no-admin
```

7. Query the token addresses and save the latest one to `CONTRACT`.
```
CONTRACT1=$(wasmd query wasm list-contract-by-code $CODE_ID1 $NODE --output json | jq -r '.contracts[-1]')
```

8. Query the token info and save the token address to `TOKEN_ADDR`.
```
TOKEN_ADDR=$(wasmd query wasm contract $CONTRACT1 $NODE --output json | jq -r '.address')
```

9. Query the token state by the state address.
```
wasmd query wasm contract-state smart $CONTRACT1 '{"token_info":{}}' $NODE
```

10. Upload `cw20_bid.wasm` to the chain.
```
RES2=$(wasmd tx wasm store artifacts/cw20_bid.wasm --from wallet1 $TXFLAG -y --output json -b block)
```

11. Get the code ID of the uploaded binary.
```
CODE_ID2=$(echo $RES2 | jq -r '.logs[0].events[-1].attributes[0].value')
```

12. Create an instance of the contract.
```
INIT='{"token_addr":"'$TOKEN_ADDR'","reserve_price":"100","increment":"10","duration_in_blocks":"50"}'
wasmd tx wasm instantiate $CODE_ID2 "$INIT" \
    --from wallet1 --label "cw20 bid" \
    $TXFLAG -y --no-admin
```

13. Query contract state addresses and save the latest one to `CONTRACT`.
```
CONTRACT2=$(wasmd query wasm list-contract-by-code $CODE_ID2 $NODE --output json | jq -r '.contracts[-1]')
```

14. Query the contract info and save the contract address to `CONTRACT_ADDR`.
```
CONTRACT_ADDR=$(wasmd query wasm contract $CONTRACT2 $NODE --output json | jq -r '.address')
```

15. Query the config by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_config"' $NODE
```

16. Place a bid using wallet2.
```
BID='{"bid":{"price":"110"}}'
wasmd tx wasm execute $CONTRACT2 "$BID" \
    --from wallet2 $TXFLAG -y
```

17. Query the bid sequence by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_bid_seq"' $NODE
```

18. Query the bid record by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '{"get_bid_record":{"id":"1"}}' $NODE
```

19. Query the best bid by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_best_bid"' $NODE
```

20. Place a bid using wallet3. This should fail since the bid price is not high enough.
```
BID='{"bid":{"price":"115"}}'
wasmd tx wasm execute $CONTRACT2 "$BID" \
    --from wallet3 $TXFLAG -y
```

21. Place a bid using wallet3 again with a higher bid price.
```
BID='{"bid":{"price":"125"}}'
wasmd tx wasm execute $CONTRACT2 "$BID" \
    --from wallet3 $TXFLAG -y
```

22. Query the bid sequence by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_bid_seq"' $NODE
```

23. Query the bid record by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '{"get_bid_record":{"id":"2"}}' $NODE
```

24. Query the best bid by the state address.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_best_bid"' $NODE
```

25. Approve the contract to transfer the CW20 token on behalf of the buyer.
```
INCREASE_ALLOWANCE='{"increase_allowance":{"spender":"'$CONTRACT_ADDR'","amount":"125","expires":{"never":{}}}}'
wasmd tx wasm execute $CONTRACT1 "$INCREASE_ALLOWANCE" \
    --from wallet3 $TXFLAG -y
```

26. Pay for the item.
```
BUY='{"receive":{"amount":"125","msg":"ImJ1eSI=","sender":"'$WALLET3'"}}'
wasmd tx wasm execute $CONTRACT2 "$BUY" \
    --from wallet3 $TXFLAG -y
```

27. Query the best bid by the state address. It should show the item has already been sold.
```
wasmd query wasm contract-state smart $CONTRACT2 '"get_best_bid"' $NODE
```

### Testing
```
cargo test
```