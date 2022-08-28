# cw20-bid
A smart contract written in Rust and built with CosmWasm for options trading.

### Version
- [Rust](https://www.rust-lang.org/): 1.61.0
- [CosmWasm](https://cosmwasm.com/): 1.0.0
- [wasmd](https://github.com/CosmWasm/wasmd): 0.27.0

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

### Testing
```
cargo test
```