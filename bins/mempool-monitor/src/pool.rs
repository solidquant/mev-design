use alloy::primitives::Address;
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::abi;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Venue {
    UniswapV2,
    UniswapV3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub id: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee: u64,
    pub venue: Venue,
    pub block: u64,
}

impl TryFrom<&Log> for Pool {
    type Error = anyhow::Error;

    fn try_from(log: &Log) -> Result<Self, Self::Error> {
        let topic = log.data().topics()[0];

        match topic {
            abi::IUniswapV2Factory::PairCreated::SIGNATURE_HASH => {
                let pair_log = abi::IUniswapV2Factory::PairCreated::decode_log(&log.inner, false)?;
                Ok(Pool {
                    id: pair_log.data.pair,
                    token0: pair_log.data.token0,
                    token1: pair_log.data.token1,
                    fee: 3000, // uniswap v2 style (0.3%)
                    venue: Venue::UniswapV2,
                    block: log.block_number.unwrap_or(0),
                })
            }
            abi::IUniswapV3Factory::PoolCreated::SIGNATURE_HASH => {
                let pool_log = abi::IUniswapV3Factory::PoolCreated::decode_log(&log.inner, false)?;
                Ok(Pool {
                    id: pool_log.data.pool,
                    token0: pool_log.data.token0,
                    token1: pool_log.data.token1,
                    fee: pool_log.data.fee.try_into()?,
                    venue: Venue::UniswapV3,
                    block: log.block_number.unwrap_or(0),
                })
            }
            _ => anyhow::bail!("Unknown event signature: {topic}"),
        }
    }
}
