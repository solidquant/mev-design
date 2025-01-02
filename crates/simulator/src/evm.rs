use std::collections::BTreeSet;
use std::sync::Arc;

use alloy::primitives::Address;
use alloy_sol_types::SolCall;
use anyhow::{anyhow, Result};
use evm_fork_db::backend::SharedBackend;
use evm_fork_db::cache::{BlockchainDb, BlockchainDbMeta};
use evm_fork_db::database::ForkedDatabase;
use evm_fork_db::types::get_db_factory;
use revm::db::WrapDatabaseRef;
use revm::primitives::state::AccountInfo;
use revm::primitives::{Account, Bytecode, ExecutionResult, Output, TransactTo, SHANGHAI, U256};
use revm::{Database, Evm};
use shared::utils::get_http_provider;
use tracing::error;

use crate::abi;
use crate::bytecode::SIMULATOR_BYTECODE;

pub struct EVM<'a> {
    backend: SharedBackend,
    fork: ForkedDatabase,
    pub evm: Evm<'a, (), WrapDatabaseRef<ForkedDatabase>>,

    weth: Address,
    block_number: u64,
    owner: Address,
    simulator: Address,
}

impl<'a> EVM<'a> {
    pub async fn new(
        rpc_url: &str,
        db_path: Option<&str>,
        static_path: Option<&str>,
        block_number: u64,
        weth: Address,
        owner: Address,
        balance: U256,
    ) -> Self {
        let http_provider = get_http_provider(rpc_url);

        let file_db_factory = db_path.map(|path| {
            get_db_factory(path, static_path.expect("static_path must be provided with db_path"))
        });

        let meta = BlockchainDbMeta {
            cfg_env: Default::default(),
            block_env: Default::default(),
            hosts: BTreeSet::from([rpc_url.to_string()]),
        };

        let db = BlockchainDb::new(meta, None);

        let backend = SharedBackend::spawn_backend(
            Arc::new(http_provider.clone()),
            file_db_factory,
            db.clone(),
            None,
        )
        .await;

        let fork = ForkedDatabase::new(backend.clone(), db.clone());

        let evm = Evm::builder()
            .with_spec_id(SHANGHAI)
            .with_ref_db(fork.clone())
            .build();

        let mut _self = Self {
            backend,
            fork,
            evm,
            weth,
            block_number,
            owner: Address::default(),
            simulator: Address::default(),
        };

        _self.set_block_number(block_number);
        _self.setup_owner(owner, balance);

        _self.simulator = _self.deploy_simulator(None);

        _self
    }

    pub fn db(&self) -> &ForkedDatabase {
        &self.fork
    }

    pub fn evm_cloned(&self) -> Evm<'_, (), WrapDatabaseRef<ForkedDatabase>> {
        Evm::builder()
            .with_spec_id(SHANGHAI)
            .with_ref_db(self.db().clone())
            .build()
    }

    pub fn weth(&self) -> Address {
        self.weth
    }

    pub fn block_number(&self) -> u64 {
        self.block_number
    }

    pub fn owner(&self) -> Address {
        self.owner
    }

    pub fn simulator(&self) -> Address {
        self.simulator
    }

    pub fn set_block_number(&mut self, block_number: u64) {
        if let Err(e) = self.backend.set_pinned_block(block_number) {
            error!("failed to set block. error={e:?}");
        }
        self.block_number = block_number;
        self.set_block_env();
    }

    pub fn set_block_env(&mut self) {
        let block_env = self.evm.block_mut();
        block_env.number = U256::from(self.block_number);
    }

    pub fn deploy_contract(
        &mut self,
        contract_addr: Option<Address>,
        bytecode_str: &str,
    ) -> Address {
        let bytes = bytecode_str.parse().unwrap();
        let code = Bytecode::new_legacy(bytes);
        let account = AccountInfo::new(U256::ZERO, 0, code.hash_slow(), code);

        let addy = match contract_addr {
            Some(addy) => addy,
            None => Address::random(),
        };

        let cache_db_mut = self.evm.db_mut().0.database_mut();
        cache_db_mut.insert_account_info(addy, account);

        addy
    }

    pub fn deploy_simulator(&mut self, contract_addr: Option<Address>) -> Address {
        self.deploy_contract(contract_addr, SIMULATOR_BYTECODE)
    }

    pub fn setup_owner(&mut self, owner: Address, balance: U256) {
        self.owner = owner;
        self.set_eth_balance(owner, balance);
    }

    pub fn basic(&mut self, target: Address) -> Result<Option<AccountInfo>> {
        self.evm
            .db_mut()
            .0
            .basic(target)
            .map_err(|e| anyhow!("failed to get basic. error={e:?}"))
    }

    pub fn get_eth_balance(&mut self, target: Address) -> U256 {
        match self.basic(target) {
            Ok(basic) => match basic {
                Some(account) => account.balance,
                None => {
                    error!("failed to get eth balance. target={}, error=no account", target);
                    U256::ZERO
                }
            },
            Err(e) => {
                error!("failed to get eth balance. target={}, error={:?}", target, e);
                U256::ZERO
            }
        }
    }

    pub fn set_eth_balance(&mut self, target: Address, balance: U256) {
        let account = match self.basic(target) {
            Ok(Some(mut account)) => {
                account.balance = balance;
                account
            }
            Ok(None) | Err(_) => AccountInfo { balance, ..Default::default() },
        };

        self.evm
            .db_mut()
            .0
            .database_mut()
            .insert_account_info(target, account);
    }

    pub fn wrap_eth(&mut self, amount: U256) -> Result<()> {
        let encoded = abi::IWETH::depositCall::new(()).abi_encode();

        let tx_env = self.evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(self.weth);
        tx_env.data = encoded.into();
        tx_env.caller = self.owner;
        tx_env.value = amount;

        let result = self.evm.transact_commit()?;

        match result {
            ExecutionResult::Halt { reason, gas_used } => {
                error!("wrap_weth halted. gas_used={}, reason={:?}", gas_used, reason);
            }
            ExecutionResult::Revert { gas_used, output } => {
                error!("wrap_weth reverted. gas_used={}, output={}", gas_used, output);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn transfer_token(
        &mut self,
        token: Address,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<()> {
        let encoded = abi::IERC20::transferCall::new((to, amount)).abi_encode();

        let tx_env = self.evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(token);
        tx_env.data = encoded.into();
        tx_env.caller = from;
        tx_env.value = U256::ZERO;

        let result = self.evm.transact_commit()?;

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

    pub fn fund_simulator(&mut self, amount: U256) -> Result<()> {
        self.wrap_eth(amount)?;
        self.transfer_token(self.weth, self.owner, self.simulator, amount)
    }

    pub fn get_token_balance(
        &mut self,
        token: Address,
        account: Address,
    ) -> Result<(U256, Account)> {
        let encoded = abi::IERC20::balanceOfCall::new((account,)).abi_encode();

        let tx_env = self.evm.tx_mut();
        tx_env.transact_to = TransactTo::Call(token);
        tx_env.data = encoded.into();
        tx_env.caller = Address::ZERO;
        tx_env.value = U256::ZERO;

        let ref_tx = self.evm.transact()?;
        let result = ref_tx.result;

        let value = match result {
            ExecutionResult::Success { output: Output::Call(value), .. } => Ok(value),
            _ => Err(anyhow!("failed to get token balance. token={}", token)),
        }?;

        let result = abi::IERC20::balanceOfCall::abi_decode_returns(&value, false)?;

        let tx_state = ref_tx.state;
        let touched_account = match tx_state.get(&token) {
            Some(state) => state,
            None => &Account::default(),
        };

        Ok((result.balance, touched_account.to_owned()))
    }
}
