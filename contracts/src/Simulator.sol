// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/console.sol";

import "./interfaces/IERC20.sol";
import "./interfaces/IERC4626.sol";
import "./interfaces/IUniswapV3Pool.sol";
import "./interfaces/IBalancerV2Vault.sol";
import "./interfaces/ICurveV2Pool.sol";

contract Simulator is IFlashLoanRecipient {
    constructor() {}

    receive() external payable {}

    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata data
    ) external {
        // no callback protection in place
        // use only for testing/simulation purposes
        uint256 amountIn = uint256(
            amount0Delta > 0 ? amount0Delta : amount1Delta
        );

        if (data.length == 64) {
            // regular v3 swap
            (address pool, address tokenIn) = abi.decode(
                data,
                (address, address)
            );
            IERC20(tokenIn).transfer(pool, amountIn);
        } else {
            // flashswap
            (address pool, address tokenIn, address tokenOut) = abi.decode(
                data,
                (address, address, address)
            );

            IERC20 lst20 = IERC20(tokenOut);
            IERC4626 lst4626 = IERC4626(tokenOut);

            uint256 lstBalance = lst20.balanceOf(address(this));
            uint256 shares = lst4626.convertToShares(lstBalance);

            // get back WETH
            lst4626.redeem(shares, address(this), address(this));

            // repay loan
            IERC20(tokenIn).transfer(pool, amountIn);
        }
    }

    error UnauthorizedCaller(address caller);
    error InvalidSellType(uint256 sellType);
    error TradeNotProfitable(int256 profit);

    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory, // feeAmounts, unused
        bytes memory data
    ) external {
        address BALANCER_VAULT = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;
        if (msg.sender != BALANCER_VAULT) revert UnauthorizedCaller(msg.sender);

        // Get initial state
        address borrowedToken = address(tokens[0]);
        uint256 initialBalance = amounts[0];

        // Decode base parameters
        (address lst, uint256 sellType) = abi.decode(data, (address, uint256));

        // Step 1: Deposit borrowed tokens into LST
        lstDeposit(lst, borrowedToken, initialBalance);
        uint256 lstBalance = IERC20(lst).balanceOf(address(this));
        console.log("LST balance: %s", lstBalance);

        // Step 2: Sell LST tokens based on sell type
        _executeSellStrategy(data, sellType, lstBalance);

        // Step 3: Calculate and log profit
        uint256 finalBalance = IERC20(borrowedToken).balanceOf(address(this));
        int256 profit = int256(finalBalance) - int256(initialBalance);
        console.log("profit: %s", profit);

        if (profit < 0) {
            revert TradeNotProfitable(profit);
        }

        // Step 4: Repay loan
        IERC20(borrowedToken).transfer(BALANCER_VAULT, initialBalance);
    }

    function _executeSellStrategy(
        bytes memory data,
        uint256 sellType,
        uint256 amount
    ) private {
        if (sellType == 0) {
            (, , address pool, bool zeroForOne) = abi.decode(
                data,
                (address, uint256, address, bool)
            );
            uniswapV3Swap(pool, zeroForOne, amount);
        } else if (sellType == 1) {
            (, , bytes32 poolId, address tokenIn, address tokenOut) = abi
                .decode(data, (address, uint256, bytes32, address, address));
            balancerV2Swap(poolId, tokenIn, tokenOut, amount);
        } else if (sellType == 2) {
            (, , address pool, uint256 tokenInIdx, uint256 tokenOutIdx) = abi
                .decode(data, (address, uint256, address, uint256, uint256));
            curveV2Swap(pool, tokenInIdx, tokenOutIdx, amount);
        } else {
            revert InvalidSellType(sellType);
        }
    }

    // Arbitrage Scenario #1
    function flashloanLstArbitrage(
        address lst,
        address tokenIn,
        uint256 amountIn,
        bytes memory sellData
    ) public {
        // performs arbitrage scenario #1
        // this is profitable if the sell price is higher than the deposit price
        // 1. Buy: deposit into LST (mint)
        // 2. Sell: sell LST from different venue
        IBalancerV2Vault vaultContract = IBalancerV2Vault(
            0xBA12222222228d8Ba445958a75a0704d566BF2C8
        );

        IERC20[] memory tokens = new IERC20[](1);
        tokens[0] = IERC20(tokenIn);

        uint256[] memory amounts = new uint256[](1);
        amounts[0] = amountIn;

        bytes memory fullData = abi.encodePacked(abi.encode(lst), sellData);

        vaultContract.flashLoan(
            IFlashLoanRecipient(address(this)),
            tokens,
            amounts,
            fullData
        );
    }

    // Arbitrage Scanario #2
    function flashswapLstArbitrage(
        address pool,
        bool zeroForOne,
        uint256 amountIn
    ) public {
        // performs arbitrage scenario #2
        // 1. Buy: loan LST from Uniswap V3 using flashswap
        // 2. Sell: withdraw WETH directly from LST contract

        IUniswapV3Pool v3Pool = IUniswapV3Pool(pool);

        address token0 = v3Pool.token0();
        address token1 = v3Pool.token1();

        (address tokenIn, address tokenOut) = zeroForOne
            ? (token0, token1)
            : (token1, token0);

        uint160 sqrtPriceLimitX96 = zeroForOne
            ? 4295128740
            : 1461446703485210103287273052203988822378723970341;

        bytes memory data = abi.encode(pool, tokenIn, tokenOut);

        v3Pool.swap(
            address(this),
            zeroForOne,
            int256(amountIn),
            sqrtPriceLimitX96,
            data
        );
    }

    function lstDeposit(address lst, address tokenIn, uint256 amountIn) public {
        IERC4626 lstContract = IERC4626(lst);
        require(lstContract.asset() == tokenIn);

        IERC20(tokenIn).approve(lst, amountIn);
        lstContract.deposit(amountIn, address(this));
    }

    // sellType = 0
    function uniswapV3Swap(
        address pool,
        bool zeroForOne,
        uint256 amountIn
    ) public {
        IUniswapV3Pool v3Pool = IUniswapV3Pool(pool);

        address tokenIn = zeroForOne ? v3Pool.token0() : v3Pool.token1();

        uint160 sqrtPriceLimitX96 = zeroForOne
            ? 4295128740
            : 1461446703485210103287273052203988822378723970341;

        bytes memory data = abi.encode(pool, tokenIn);

        v3Pool.swap(
            address(this),
            zeroForOne,
            int256(amountIn),
            sqrtPriceLimitX96,
            data
        );
    }

    // sellType = 1
    function balancerV2Swap(
        bytes32 poolId,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) public {
        address vault = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;

        IERC20 tokenInContract = IERC20(tokenIn);
        tokenInContract.approve(vault, 0xFFFFFFFFFFFFFFFFFFFFFFFF);

        IBalancerV2Vault vaultContract = IBalancerV2Vault(vault);

        vaultContract.swap(
            IBalancerV2Vault.SingleSwap(
                poolId,
                IBalancerV2Vault.SwapKind.GIVEN_IN,
                IAsset(tokenIn),
                IAsset(tokenOut),
                amountIn,
                new bytes(0)
            ),
            IBalancerV2Vault.FundManagement(
                address(this),
                false,
                payable(address(this)),
                false
            ),
            0,
            11533977638873292903519766084849772071321814878804040558617845282038221897728
        );
    }

    // sellType = 2
    function curveV2Swap(
        address pool,
        uint256 tokenInIdx,
        uint256 tokenOutIdx,
        uint256 amountIn
    ) public {
        ICurveV2Pool v2Pool = ICurveV2Pool(pool);

        address tokenIn = v2Pool.coins(tokenInIdx);

        IERC20 tokenInContract = IERC20(tokenIn);
        tokenInContract.approve(pool, amountIn);

        v2Pool.exchange(tokenInIdx, tokenOutIdx, amountIn, 0);
    }
}
