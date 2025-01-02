use alloy::primitives::Address;
use alloy_sol_types::SolCall;
use anyhow::{anyhow, Result};
use revm::primitives::{ExecutionResult, Output, TransactTo, U256};

use crate::abi;
use crate::evm::EVM;

pub trait UniswapV3PoolContract {
    fn token0(&mut self, contract_address: Address) -> Result<Address>;

    fn token1(&mut self, contract_address: Address) -> Result<Address>;
}

impl UniswapV3PoolContract for EVM<'_> {
    fn token0(&mut self, contract_address: Address) -> Result<Address> {
        let owner = self.owner();

        let encoded = abi::IUniswapV3Pool::token0Call::new(()).abi_encode();

        let evm = &mut self.evm;

        let tx_env = evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(contract_address);
        tx_env.data = encoded.into();
        tx_env.caller = owner;
        tx_env.value = U256::ZERO;

        let ref_tx = evm.transact()?;
        let result = ref_tx.result;

        let value = match result {
            ExecutionResult::Success { output: Output::Call(value), .. } => Ok(value),
            _ => Err(anyhow!("failed to get token0. pool={}", contract_address)),
        }?;

        let result = abi::IUniswapV3Pool::token0Call::abi_decode_returns(&value, false)?;

        Ok(result._0)
    }

    fn token1(&mut self, contract_address: Address) -> Result<Address> {
        let owner = self.owner();

        let encoded = abi::IUniswapV3Pool::token1Call::new(()).abi_encode();

        let evm = &mut self.evm;

        let tx_env = evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(contract_address);
        tx_env.data = encoded.into();
        tx_env.caller = owner;
        tx_env.value = U256::ZERO;

        let ref_tx = evm.transact()?;
        let result = ref_tx.result;

        let value = match result {
            ExecutionResult::Success { output: Output::Call(value), .. } => Ok(value),
            _ => Err(anyhow!("failed to get token0. pool={}", contract_address)),
        }?;

        let result = abi::IUniswapV3Pool::token1Call::abi_decode_returns(&value, false)?;

        Ok(result._0)
    }
}
