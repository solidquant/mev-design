pub(crate) mod pool;
pub(crate) mod utils;

use std::path::Path;

use alloy::providers::ext::DebugApi;
use alloy::providers::Provider;
use alloy::sol_types::SolEvent;
use alloy_rpc_types::transaction::TransactionRequest;
use alloy_rpc_types_eth::BlockNumberOrTag;
use alloy_rpc_types_trace::geth::{
    CallConfig, CallFrame, CallLogFrame, GethDebugTracingCallOptions, GethTrace,
};
use anyhow::Result;
use futures_util::StreamExt;
use shared::utils::{get_env, get_ws_provider};
use simulator::abi;
use tracing::info;

use crate::utils::load_pools;

fn collect_logs(frame: &CallFrame) -> Vec<CallLogFrame> {
    std::iter::once(frame)
        .flat_map(|f| {
            f.logs
                .iter()
                .cloned()
                .chain(f.calls.iter().flat_map(collect_logs))
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables.
    dotenv::dotenv().ok();

    // Setup tracing.
    let log_dir = Path::new("logs");
    let _guard = shared::logging::setup_tracing(Some(&log_dir), Some("mempool-monitor.log"));

    info!("Starting mempool monitor");

    // Log panics as errors.
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ::tracing::error!("Application panic; panic={panic_info:?}");

        default_panic(panic_info);
    }));

    let rpc_wss_url = get_env("RPC_WSS_URL");
    info!("RPC WSS URL: {}", rpc_wss_url);

    let provider = get_ws_provider(&rpc_wss_url).await;

    // Load all Uniswap V2, V3 pools.
    let pools = load_pools(&rpc_wss_url, 0).await.unwrap();
    info!("Loaded {} pools", pools.len());

    let sub = provider.subscribe_pending_transactions().await?;
    let mut stream = sub.into_stream();

    while let Some(tx_hash) = stream.next().await {
        if let Ok(Some(tx)) = provider.get_transaction_by_hash(tx_hash).await {
            println!("\nTx hash: {}", tx_hash);

            let trace_tx = TransactionRequest::from_transaction(tx);

            let mut config = GethDebugTracingCallOptions::default();

            let mut call_config = CallConfig::default();
            call_config = call_config.with_log();

            config.tracing_options.tracer =
                Some(alloy_rpc_types_trace::geth::GethDebugTracerType::BuiltInTracer(
                    alloy_rpc_types_trace::geth::GethDebugBuiltInTracerType::CallTracer,
                ));

            config.tracing_options.tracer_config =
                serde_json::to_value(call_config).unwrap().into();

            if let Ok(trace) = provider
                .debug_trace_call(trace_tx, BlockNumberOrTag::Latest.into(), config)
                .await
            {
                if let GethTrace::CallTracer(frame) = trace {
                    let logs = collect_logs(&frame);

                    for log in logs.iter() {
                        if let Some(topics) = &log.topics {
                            let topic = topics[0];

                            let alloy_log = alloy_primitives::Log {
                                address: log.address.unwrap(),
                                data: alloy_primitives::LogData::new(
                                    log.topics.clone().unwrap(),
                                    log.data.clone().unwrap(),
                                )
                                .unwrap(),
                            };

                            match topic {
                                abi::IERC20::Transfer::SIGNATURE_HASH => {
                                    let transfer_log =
                                        abi::IERC20::Transfer::decode_log(&alloy_log, false);

                                    info!("Transfer: {:?}", transfer_log);
                                }

                                abi::CrocSwapDex::CrocSwap::SIGNATURE_HASH => {
                                    let swap_log =
                                        abi::CrocSwapDex::CrocSwap::decode_log(&alloy_log, false);

                                    info!("Croc: {:?}", swap_log);
                                }

                                abi::IUniswapV2Pair::Swap::SIGNATURE_HASH => {
                                    let swap_log =
                                        abi::IUniswapV2Pair::Swap::decode_log(&alloy_log, false);

                                    info!("V2: {:?}", swap_log);
                                }

                                abi::IUniswapV3Pool::Swap::SIGNATURE_HASH => {
                                    let swap_log =
                                        abi::IUniswapV3Pool::Swap::decode_log(&alloy_log, false);

                                    info!("V3: {:?}", swap_log);
                                }

                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
