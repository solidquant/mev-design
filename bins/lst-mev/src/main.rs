use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use alloy::primitives::Address;
use anyhow::Result;
use revm::primitives::U256;
use shared::utils::get_env;
use simulator::evm::EVM;
use simulator::traits::{SimulatorContract, UniswapV3PoolContract};
use tracing::info;

#[derive(Debug, Clone)]
struct Optimized {
    pub optimized_in: u128,
    pub optimized_out: u128,
}

async fn simulate(
    rpc_https_url: &str,
    target_block_number: u64,
    weth: Address,
    target_uniswap_v3_pool: Address,
    zfo: bool,
    amount_in: u128,
) -> Result<u128> {
    let owner = Address::random();

    let mut evm = EVM::new(
        &rpc_https_url,
        None,
        None,
        target_block_number,
        weth,
        owner,
        U256::from(10_u64.pow(18)), // 1 ETH
    )
    .await;

    let balance_before = evm.get_token_balance(weth, evm.simulator()).unwrap().0;

    // Perform flashswap arbitrage.
    evm.flashswap_lst_arbitrage(target_uniswap_v3_pool, zfo, U256::from(amount_in))?;

    let balance_after = evm.get_token_balance(weth, evm.simulator()).unwrap().0;

    let profit = balance_after.saturating_sub(balance_before);

    match profit.try_into() {
        Ok(profit_u64) => Ok(profit_u64),
        Err(_) => {
            info!("Profit too large for u128, returning 0");
            Ok(0)
        }
    }
}

// Quadratic search for optimal amount_in.
async fn optimize_arbitrage(
    rpc_https_url: &str,
    target_block_number: u64,
    weth: Address,
    target_uniswap_v3_pool: Address,
    zfo: bool,
) -> Result<Optimized> {
    let intervals = 10;
    let tolerance = 10_u128.pow(15); // 0.001 ETH
    let ceiling = 10_u128.pow(18) * 1000; // 1000 ETH

    let mut min_amount_in = 0; // 0 ETH
    let mut max_amount_in = ceiling;
    let mut optimized_in = 0;
    let mut max_profit = 0;

    while max_amount_in - min_amount_in > tolerance {
        let step = (max_amount_in - min_amount_in) / intervals;
        if step == 0 {
            break;
        }

        let mut best_local_profit = 0;
        let mut best_local_amount_in = min_amount_in;

        for i in 0..=intervals {
            let amount_in = std::cmp::min(min_amount_in + i * step, ceiling);

            let s = Instant::now();
            let profit = simulate(
                rpc_https_url,
                target_block_number,
                weth,
                target_uniswap_v3_pool,
                zfo,
                amount_in,
            )
            .await
            .unwrap_or(0);
            let took = s.elapsed().as_millis();
            info!("amount_in={amount_in}, profit={profit}, took={took}ms");

            if profit > best_local_profit {
                best_local_profit = profit;
                best_local_amount_in = amount_in;
            }

            if profit > max_profit {
                max_profit = profit;
                optimized_in = amount_in;
            }

            if amount_in == ceiling {
                break;
            }
        }

        if best_local_amount_in == min_amount_in {
            min_amount_in = best_local_amount_in;
            max_amount_in = std::cmp::min(best_local_amount_in + step, ceiling);
        } else if best_local_amount_in == max_amount_in {
            min_amount_in = max_amount_in.saturating_sub(step);
            // NB: Intentionally leave max_amount_in unchanged.
        } else {
            min_amount_in = best_local_amount_in.saturating_sub(step);
            max_amount_in = std::cmp::min(best_local_amount_in + step, ceiling);
        }
    }

    let optimized_in: u128 = optimized_in.try_into().unwrap_or(0);
    let optimized_out: u128 = max_profit.try_into().unwrap_or(0);

    Ok(Optimized { optimized_in, optimized_out })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables.
    dotenv::dotenv().ok();

    // Setup tracing.
    let log_dir = Path::new("logs");
    let _guard = shared::logging::setup_tracing(Some(&log_dir), Some("lst-mev.log"));

    info!("Starting LST MEV simulation");

    // Log panics as errors.
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ::tracing::error!("Application panic; panic={panic_info:?}");

        default_panic(panic_info);
    }));

    let rpc_https_url = get_env("RPC_HTTPS_URL");
    info!("RPC HTTPS URL: {}", rpc_https_url);

    let target_block_number = 18732930;
    info!("Target block number: {}", target_block_number);

    let weth = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();

    let target_uniswap_v3_pool =
        Address::from_str("0xDeBead39628F93905dfc3E88003af40bf11189b0").unwrap();

    let owner = Address::random();

    let mut evm = EVM::new(
        &rpc_https_url,
        None,
        None,
        target_block_number,
        weth,
        owner,
        U256::from(10_u64.pow(18)), // 1 ETH
    )
    .await;

    let token0 = evm.token0(target_uniswap_v3_pool).unwrap();
    let zfo = token0 == weth;

    let optimized =
        optimize_arbitrage(&rpc_https_url, target_block_number, weth, target_uniswap_v3_pool, zfo)
            .await
            .unwrap();

    info!("Optimized: {:?}", optimized);

    info!("Optimized amount in: {}", optimized.optimized_in);
    info!("Optimized profit: {}", optimized.optimized_out);

    Ok(())
}
