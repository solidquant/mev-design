use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use alloy::sol_types::SolEvent;
use alloy_provider::Provider;
use anyhow::Result;
use csv::{Reader, Writer};
use shared::utils::{get_block_range, get_logs, get_ws_provider};
use tracing::info;

use crate::abi;
use crate::pool::Pool;

fn save_to_csv(pools: &[Pool], path: &Path) -> Result<()> {
    let mut writer = Writer::from_path(path)?;

    for pool in pools {
        writer.serialize(pool)?;
    }

    writer.flush()?;
    Ok(())
}

fn load_from_csv(path: &Path) -> Result<Vec<Pool>> {
    let mut reader = Reader::from_path(path)?;
    let mut pools = Vec::new();

    for result in reader.deserialize() {
        let pool: Pool = result?;
        pools.push(pool);
    }

    Ok(pools)
}

pub(crate) async fn load_pools(wss_url: &str, from_block: u64) -> Result<Vec<Pool>> {
    let provider = Arc::new(get_ws_provider(wss_url).await);
    info!("connected to provider");

    let cache_dir = Path::new("cache");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
        info!("Created cache directory at {:?}", cache_dir);
    }

    let pools_cache_path = cache_dir.join("pools.csv");
    let pools = if pools_cache_path.exists() { load_from_csv(&pools_cache_path)? } else { vec![] };

    let start_block = pools
        .iter()
        .map(|pool| pool.block)
        .max()
        .map_or(from_block, |block| block + 1);

    let end_block = provider.get_block_number().await?;

    if start_block >= end_block {
        info!("No new blocks to scan");
        return Ok(pools);
    }

    info!("Scanning blocks {start_block} to {end_block}");
    let mut pools = pools;
    let events = [
        abi::IUniswapV2Factory::PairCreated::SIGNATURE,
        abi::IUniswapV3Factory::PoolCreated::SIGNATURE,
    ];

    // Process blocks in chunks
    const CHUNK_SIZE: u64 = 10_000;
    for (chunk_start, chunk_end) in get_block_range(start_block, end_block, CHUNK_SIZE) {
        let timer = Instant::now();

        match get_logs(provider.clone(), chunk_start, chunk_end, None, &events).await {
            Ok(logs) => {
                info!("Processing blocks {chunk_start}-{chunk_end}: found {} logs", logs.len());

                let new_pools: Vec<_> = logs
                    .iter()
                    .filter_map(|log| {
                        Pool::try_from(log)
                            .map_err(|e| info!("Failed to parse pool from log: {e}"))
                            .ok()
                    })
                    .collect();

                if !new_pools.is_empty() {
                    info!(
                        "Added {} new pools in {}ms",
                        new_pools.len(),
                        timer.elapsed().as_millis()
                    );
                    pools.extend(new_pools);
                }
            }
            Err(e) => {
                info!("Failed to fetch logs for blocks {chunk_start}-{chunk_end}: {e}");
                continue;
            }
        }
    }

    // Save results
    if let Err(e) = save_to_csv(&pools, &pools_cache_path) {
        info!("Failed to save pools to cache: {e}");
    } else {
        info!("Saved {} pools to {:?}", pools.len(), pools_cache_path);
    }

    Ok(pools)
}
