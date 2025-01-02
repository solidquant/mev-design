use alloy::sol;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IERC20 {
        event Transfer(address indexed from, address indexed to, uint256 value);

        function name() external view returns (string);

        function symbol() external view returns (string);

        function decimals() external view returns (uint8);

        function totalSupply() external view returns (uint256);

        function balanceOf(address account) external view returns (uint256 balance);

        function transfer(address to, uint value) external returns (bool success);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IWETH {
        function deposit() external payable;

        function transfer(address to, uint value) external returns (bool success);

        function withdraw(uint amount) external;

        function balanceOf(address account) external view returns (uint256);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IERC4626 {
        event Deposit(
            address indexed caller,
            address indexed owner,
            uint256 assets,
            uint256 shares
        );

        event Withdraw(
            address indexed caller,
            address indexed receiver,
            address indexed owner,
            uint256 assets,
            uint256 shares
        );

        function convertToShares(uint256 assets) external view returns (uint256);

        function convertToAssets(uint256 shares) external view returns (uint256);

        function asset() external view returns (address);

        function deposit(
            uint256 assets,
            address receiver
        ) external returns (uint256 shares);

        function mint(
            uint256 shares,
            address receiver
        ) external returns (uint256 assets);

        function withdraw(
            uint256 assets,
            address receiver,
            address owner
        ) external returns (uint256 shares);

        function redeem(
            uint256 shares,
            address receiver,
            address owner
        ) external returns (uint256 assets);

        function previewMint(uint256 shares) external view returns (uint256);

        function previewDeposit(uint256 assets) external view returns (uint256);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract ICurveV2Pool {
        function get_dy(
            uint256 i,
            uint256 j,
            uint256 dx
        ) external returns (uint256);

        function exchange(
            uint256 i,
            uint256 j,
            uint256 dx,
            uint256 min_dy
        ) external;

        function coins(uint256 index) external returns (address);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IUniswapV3Pool {
        event Swap(
            address indexed sender,
            address indexed recipient,
            int256 amount0,
            int256 amount1,
            uint160 sqrtPriceX96,
            uint128 liquidity,
            int24 tick
        );

        event Burn(
            address indexed owner,
            int24 indexed tickLower,
            int24 indexed tickUpper,
            uint128 amount,
            uint256 amount0,
            uint256 amount1
        );

        event Mint(
            address sender,
            address indexed owner,
            int24 indexed tickLower,
            int24 indexed tickUpper,
            uint128 amount,
            uint256 amount0,
            uint256 amount1
        );

        function token0() external view returns (address);

        function token1() external view returns (address);

        function fee() external view returns (uint24);

        function tickSpacing() external view returns (int24);

        function liquidity() external view returns (uint128);

        function slot0()
            external
            view
            returns (
                uint160 sqrtPriceX96,
                int24 tick,
                uint16 observationIndex,
                uint16 observationCardinality,
                uint16 observationCardinalityNext,
                uint8 feeProtocol,
                bool unlocked
            );

        function ticks(
            int24 tick
        )
            external
            view
            returns (
                uint128 liquidityGross,
                int128 liquidityNet,
                uint256 feeGrowthOutside0X128,
                uint256 feeGrowthOutside1X128,
                int56 tickCumulativeOutside,
                uint160 secondsPerLiquidityOutsideX128,
                uint32 secondsOutside,
                bool initialized
            );

        function swap(
            address recipient,
            bool zeroForOne,
            int256 amountSpecified,
            uint160 sqrtPriceLimitX96,
            bytes calldata data
        ) external returns (int256 amount0, int256 amount1);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract CrocSwapDex {
        event CrocSwap(
            address indexed base,
            address indexed quote,
            uint256 poolIdx,
            bool isBuy,
            bool inBaseQty,
            uint128 qty,
            uint16 tip,
            uint128 limitPrice,
            uint128 minOut,
            uint8 reserveFlags,
            int128 baseFlow,
            int128 quoteFlow
        );
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IUniswapV2Factory {
        event PairCreated(address indexed token0, address indexed token1, address pair, uint);
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IUniswapV2Pair {
        event Swap(
            address indexed sender,
            uint amount0In,
            uint amount1In,
            uint amount0Out,
            uint amount1Out,
            address indexed to
        );

        function token0() external view returns (address);

        function token1() external view returns (address);

        function getReserves() external view returns (
            uint112 reserve0,
            uint112 reserve1,
            uint32 blockTimestampLast
        );
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract IUniswapV3Factory {
        event PoolCreated(
            address indexed token0,
            address indexed token1,
            uint24 indexed fee,
            int24 tickSpacing,
            address pool
        );
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc)]
    contract Simulator {
        function flashswapLstArbitrage(
            address pool,
            bool zeroForOne,
            uint256 amountIn
        ) external;
    }
}
