use alloy::primitives::Address;
use alloy_sol_types::SolCall;
use anyhow::Result;
use revm::primitives::{ExecutionResult, TransactTo, U256};
use tracing::error;

use crate::abi;
use crate::evm::EVM;

pub trait SimulatorContract {
    fn flashswap_lst_arbitrage(&mut self, pool: Address, zfo: bool, amount_in: U256) -> Result<()>;
}

impl SimulatorContract for EVM<'_> {
    fn flashswap_lst_arbitrage(&mut self, pool: Address, zfo: bool, amount_in: U256) -> Result<()> {
        let owner = self.owner();
        let simulator = self.simulator();

        let encoded =
            abi::Simulator::flashswapLstArbitrageCall::new((pool, zfo, amount_in)).abi_encode();

        let evm = &mut self.evm;

        let tx_env = evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(simulator);
        tx_env.data = encoded.into();
        tx_env.caller = owner;
        tx_env.value = U256::ZERO;

        let result = evm.transact_commit()?;

        match result {
            ExecutionResult::Halt { reason, gas_used } => {
                error!("transfer_token halted. gas_used={}, reason={:?}", gas_used, reason);
            }
            ExecutionResult::Revert { gas_used, output } => {
                error!("transfer_token reverted. gas_used={}, output={}", gas_used, output);
            }
            _ => {}
        }

        Ok(())
    }
}
