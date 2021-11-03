use std::io::Stderr;
use std::iter::Map;

use cosmwasm_std::{Addr, Api, BalanceResponse, BankMsg, BankQuery, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, from_binary, MessageInfo, Querier, QueryRequest, Response, StdError, StdResult, Storage, SubMsg, to_binary, Uint128, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;

use crate::error::ContractError;
use crate::msg::{AddressPortion, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:terranauts-royalty";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
    )
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Distribute { recipients } => try_distribute(deps, info, recipients),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance {} => to_binary(&query_balance(deps, _env.contract.address.into_string(), "uluna".to_string())?),
    }
}

pub fn query_balance(
    deps: Deps,
    account_addr: String,
    denom: String,
) -> StdResult<BalanceResponse> {
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr,
        denom,
    }))?;

    Ok(balance.into())
}

pub fn try_distribute(deps: DepsMut, info: MessageInfo, recipients: Vec<AddressPortion>) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    if info.sender.clone().into_string() != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    if recipients.len() <= 0 {
        return Err(ContractError::NoRecipients {});
    }

    let mut submsg: Vec<SubMsg> = Vec::new();

    for recipient in recipients {
        let addr = recipient.addr;
        let amount = recipient.amount;

        let mut vec_coin: Vec<Coin> = Vec::new();
        let mut recip_coin: Coin = Coin::new(amount.u128(), "uluna");
        vec_coin.push(recip_coin);

        submsg.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: addr,
            amount: vec_coin,
        })));
    }

    Ok(Response::new().add_submessages(submsg))
}

#[cfg(test)]
mod tests {
    use std::io::Stderr;

    use cosmwasm_std::{coins, from_binary};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_distribute() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "creator";

        let config = Config {
            owner: Addr::unchecked(owner.to_string()),
        };
        CONFIG.save(deps.as_mut().storage, &config).unwrap();

        let recipients: Vec<AddressPortion> = vec![];
        let msg = ExecuteMsg::Distribute { recipients };

        // fail if the sender is not the owner
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone());
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        let info = mock_info("creator", &[]);
        let err_res =
            execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

        match err_res {
            Err(ContractError::NoRecipients {}) => {}

            _ => panic!("Must error when no recipients received"),
        }

        let info = mock_info("creator", &[]);

        let msg = ExecuteMsg::Distribute {
            recipients: vec![
                AddressPortion {
                    addr: "address1".to_string(),
                    amount: Uint128::from(10 as u128),
                },
                AddressPortion {
                    addr: "address2".to_string(),
                    amount: Uint128::from(20 as u128),
                },
            ]
        };

        let res =
            execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "address1".to_string(),
                amount: vec![Coin::new(10, "uluna")],
            }))
        );
        assert_eq!(
            res.messages[1],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "address2".to_string(),
                amount: vec![Coin::new(20, "uluna")],
            }))
        );
    }
}
