#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, Uint64,
};
use cw2::set_contract_version;
use cw20::{Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{BidResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{BestBid, BidRecord, Config, BEST_BID, BID_RECORDS, BID_SEQ, CONFIG};

const CONTRACT_NAME: &str = "crates.io:cw20-bid";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let timeout = env
        .block
        .height
        .checked_add(msg.duration_in_blocks.u64())
        .expect("Failed to add block height");
    let config = Config {
        seller: info.sender.clone(),
        token_addr: deps.api.addr_validate(msg.token_addr.as_str())?,
        reserve_price: msg.reserve_price,
        increment: msg.increment,
        timeout: Uint64::new(timeout),
    };
    CONFIG.save(deps.storage, &config)?;

    BID_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("seller", info.sender)
        .add_attribute("token_addr", msg.token_addr)
        .add_attribute("reserve_price", msg.reserve_price)
        .add_attribute("increment", msg.increment)
        .add_attribute("timeout", timeout.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bid { price } => execute_bid(deps, env.block.height, info, price),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env.block.height, info, msg),
    }
}

pub fn execute_bid(
    deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    price: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if block_height >= config.timeout.u64() {
        return Err(ContractError::CustomError {
            val: format!("Auction closed"),
        });
    }
    if price < config.reserve_price {
        return Err(ContractError::CustomError {
            val: format!(
                "Bid price lower than reserve price, bid price: {:?}, reserve price: {:?}",
                price, config.reserve_price
            ),
        });
    }

    let id = BID_SEQ.load(deps.storage)?;
    let best_price = if id == 0u64 {
        config.reserve_price
    } else {
        let best_bid = BEST_BID.load(deps.storage)?;
        if price <= best_bid.bid_record.price {
            return Err(ContractError::CustomError {
                val: format!(
                    "Bid price not greater than best price, bid price: {:?}, best price: {:?}",
                    price, best_bid.bid_record.price
                ),
            });
        }
        best_bid.bid_record.price
    };
    let increment = price
        .checked_sub(best_price)
        .expect("Failed to get bid increment");
    if increment < config.increment {
        return Err(ContractError::CustomError {
            val: format!(
                "Bid increment too low, increment: {:?}, minimum increment: {:?}",
                increment, config.increment
            ),
        });
    }

    let next_id = Uint64::new(id)
        .checked_add(Uint64::new(1))
        .expect("Failed to increment the sequence");
    BID_SEQ.save(deps.storage, &next_id.u64())?;

    let bid_record = BidRecord {
        buyer: info.sender.clone(),
        price,
    };
    BID_RECORDS.save(deps.storage, next_id.u64(), &bid_record)?;

    let best_bid = BestBid {
        id: next_id,
        bid_record: BidRecord {
            buyer: info.sender.clone(),
            price,
        },
        sold: false,
    };
    BEST_BID.save(deps.storage, &best_bid)?;

    Ok(Response::new()
        .add_attribute("action", "execute_bid")
        .add_attribute("id", next_id)
        .add_attribute("buyer", info.sender)
        .add_attribute("price", price))
}

pub fn execute_receive(
    deps: DepsMut,
    block_height: u64,
    info: MessageInfo,
    wrapped_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if block_height < config.timeout.u64() {
        return Err(ContractError::CustomError {
            val: format!("Auction not yet closed"),
        });
    }

    let msg: ReceiveMsg = from_binary(&wrapped_msg.msg)?;
    match msg {
        ReceiveMsg::Buy => receive_buy(deps, config.token_addr, wrapped_msg.amount, info.sender, config.seller),
    }
}

pub fn receive_buy(
    deps: DepsMut,
    token_addr: Addr,
    amount: Uint128,
    buyer: Addr,
    seller: Addr,
) -> Result<Response, ContractError> {
    let mut best_bid = BEST_BID.load(deps.storage)?;
    if best_bid.sold {
        return Err(ContractError::CustomError {
            val: format!("Item already sold"),
        });
    }
    if buyer != best_bid.bid_record.buyer {
        return Err(ContractError::Unauthorized {});
    }
    if amount < best_bid.bid_record.price {
        return Err(ContractError::CustomError {
            val: format!(
                "Amount lower than bid price, amount: {:?}, bid price: {:?}",
                amount, best_bid.bid_record.price
            ),
        });
    }

    best_bid.sold = true;
    BEST_BID.save(deps.storage, &best_bid)?;

    let cw20 = Cw20Contract(token_addr);
    let msg = cw20.call(Cw20ExecuteMsg::TransferFrom {
        owner: buyer.clone().into_string(),
        recipient: seller.into_string(),
        amount,
    })?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "receive_buy")
        .add_attribute("id", best_bid.id)
        .add_attribute("buyer", buyer)
        .add_attribute("amount", amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetBidSeq => to_binary(&BID_SEQ.load(deps.storage)?),
        QueryMsg::GetBidRecord { id } => to_binary(&query_bid(deps, id)?),
        QueryMsg::GetBestBid => to_binary(&BEST_BID.load(deps.storage)?),
    }
}

fn query_bid(deps: Deps, id: Uint64) -> StdResult<BidResponse> {
    let bid_record = BID_RECORDS.load(deps.storage, id.u64())?;
    Ok(BidResponse {
        buyer: bid_record.buyer.into_string(),
        price: bid_record.price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let token_addr = String::from("cw20 token");
        let reserve_price = Uint128::new(100);
        let increment = Uint128::new(10);
        let duration_in_blocks = Uint64::new(200);
        let msg = InstantiateMsg {
            token_addr,
            reserve_price,
            increment,
            duration_in_blocks,
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();
        env.block.height = 200_000;
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 6);

        let res = query(deps.as_ref(), env.clone(), QueryMsg::GetConfig).unwrap();
        let config: Config = from_binary(&res).unwrap();
        assert_eq!(config.seller, "creator");
        assert_eq!(config.token_addr, "cw20 token");
        assert_eq!(config.reserve_price, reserve_price);
        assert_eq!(config.increment, increment);
        assert_eq!(config.timeout, Uint64::new(200_200));

        let res = query(deps.as_ref(), env, QueryMsg::GetBidSeq).unwrap();
        let bid_seq: u64 = from_binary(&res).unwrap();
        assert_eq!(bid_seq, 0u64);
    }

    #[test]
    fn test_bid() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            token_addr: String::from("cw20 token"),
            reserve_price: Uint128::new(100),
            increment: Uint128::new(10),
            duration_in_blocks: Uint64::new(200),
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();
        env.block.height = 200_000;
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Bid {
            price: Uint128::new(80),
        };
        let info = mock_info("buyer", &[]);
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Bid price lower than reserve price")),
            e => panic!("unexpected error: {}", e),
        }

        let msg = ExecuteMsg::Bid {
            price: Uint128::new(109),
        };
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Bid increment too low")),
            e => panic!("unexpected error: {}", e),
        }

        let bid_price = Uint128::new(110);
        let msg = ExecuteMsg::Bid { price: bid_price };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(res.attributes.len(), 4);

        let res = query(deps.as_ref(), env.clone(), QueryMsg::GetBidSeq).unwrap();
        let bid_seq: u64 = from_binary(&res).unwrap();
        assert_eq!(bid_seq, 1u64);

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetBidRecord {
                id: Uint64::new(bid_seq),
            },
        )
        .unwrap();
        let bid_record: BidRecord = from_binary(&res).unwrap();
        assert_eq!(bid_record.buyer, "buyer");
        assert_eq!(bid_record.price, bid_price);

        let res = query(deps.as_ref(), env.clone(), QueryMsg::GetBestBid).unwrap();
        let best_bid: BestBid = from_binary(&res).unwrap();
        assert_eq!(best_bid.id, Uint64::new(1));
        assert_eq!(best_bid.bid_record.buyer, "buyer");
        assert_eq!(best_bid.bid_record.price, bid_price);
        assert_eq!(best_bid.sold, false);

        let err = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Bid price not greater than best price")),
            e => panic!("unexpected error: {}", e),
        }

        let msg = ExecuteMsg::Bid {
            price: Uint128::new(130),
        };
        let mut env = mock_env();
        env.block.height = 200_200;
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Auction closed")),
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn test_buy() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            token_addr: String::from("cw20 token"),
            reserve_price: Uint128::new(100),
            increment: Uint128::new(10),
            duration_in_blocks: Uint64::new(200),
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();
        env.block.height = 200_000;
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Bid {
            price: Uint128::new(110),
        };
        let buyer_info = mock_info("buyer", &[]);
        execute(deps.as_mut(), env.clone(), buyer_info.clone(), msg).unwrap();

        let proper_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("buyer"),
            amount: Uint128::new(110),
            msg: to_binary(&ReceiveMsg::Buy).unwrap(),
        });
        let err = execute(
            deps.as_mut(),
            env.clone(),
            buyer_info.clone(),
            proper_msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Auction not yet closed")),
            e => panic!("unexpected error: {}", e),
        }

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("anyone"),
            amount: Uint128::new(110),
            msg: to_binary(&ReceiveMsg::Buy).unwrap(),
        });
        let info = mock_info("anyone", &[]);
        let mut env = mock_env();
        env.block.height = 200_300;
        let err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("buyer"),
            amount: Uint128::new(105),
            msg: to_binary(&ReceiveMsg::Buy).unwrap(),
        });
        let err = execute(deps.as_mut(), env.clone(), buyer_info.clone(), msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Amount lower than bid price")),
            e => panic!("unexpected error: {}", e),
        }

        let res = execute(
            deps.as_mut(),
            env.clone(),
            buyer_info.clone(),
            proper_msg.clone(),
        )
        .unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.attributes.len(), 4);

        let res = query(deps.as_ref(), env.clone(), QueryMsg::GetBestBid).unwrap();
        let best_bid: BestBid = from_binary(&res).unwrap();
        assert_eq!(best_bid.sold, true);

        let err = execute(deps.as_mut(), env, buyer_info, proper_msg).unwrap_err();
        match err {
            ContractError::CustomError { val } => assert!(val.contains("Item already sold")),
            e => panic!("unexpected error: {}", e),
        }
    }
}
