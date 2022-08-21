use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Timestamp, Uint128, Uint64};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub seller: Addr,
    pub reserve_price: Uint128,
    pub increment: Uint128,
    pub timeout: Timestamp,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidRecord {
    pub buyer: Addr,
    pub price: Uint128,
}

pub const BID_SEQ: Item<u64> = Item::new("bid_seq");
pub const BID_RECORDS: Map<u64, BidRecord> = Map::new("bid_records");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BestBid {
    pub id: Uint64,
    pub bid_record: BidRecord,
    pub sold: bool,
}

pub const BEST_BID: Item<BestBid> = Item::new("best_bid");
