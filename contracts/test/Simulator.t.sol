// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";

import "../src/Simulator.sol";
import "../src/interfaces/IERC20.sol";
import "../src/interfaces/IWETH.sol";
import "../src/interfaces/IMevEth.sol";
import "../src/interfaces/IUniswapV3Pool.sol";
import "../src/interfaces/IBalancerV2Vault.sol";
import "../src/interfaces/IBalancerV2Pool.sol";
import "../src/interfaces/ICurveV2Pool.sol";

// forge build
// forge test --fork-url http://localhost:8545 --via-ir -vv
contract SimulatorTest is Test {
    Simulator public simulator;

    IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);

    function setUp() public {
        simulator = new Simulator();
    }

    function testLstDeposit() public {
        // wrap ETH and send to simulator contract
        uint256 amountIn = 100000000000000000; // 0.1 ETH
        weth.deposit{value: amountIn}();

        weth.transfer(address(simulator), amountIn);

        // check that WETH has been received
        uint256 simulatorWethBalance = weth.balanceOf(address(simulator));
        console.log("simulator weth balance: %s", simulatorWethBalance);

        address targetLst = 0x24Ae2dA0f361AA4BE46b48EB19C91e02c5e4f27E;

        // simulate LST deposit
        IERC20 tokenOutContract = IERC20(targetLst);

        uint256 balanceBefore = tokenOutContract.balanceOf(address(simulator));

        simulator.lstDeposit(targetLst, address(weth), amountIn);

        uint256 balanceAfter = tokenOutContract.balanceOf(address(simulator));

        console.log("balance before: %s", balanceBefore);
        console.log("balance after: %s", balanceAfter);

        assert(balanceAfter > balanceBefore);
    }

    function testUniswapV3Swap() public {
        // wrap ETH and send to simulator contract
        uint256 amountIn = 100000000000000000; // 0.1 ETH
        weth.deposit{value: amountIn}();

        weth.transfer(address(simulator), amountIn);

        // check that WETH has been received
        uint256 simulatorWethBalance = weth.balanceOf(address(simulator));
        console.log("simulator weth balance: %s", simulatorWethBalance);

        address targetUniswapV3Pool = 0xDeBead39628F93905dfc3E88003af40bf11189b0;

        IUniswapV3Pool v3Pool = IUniswapV3Pool(targetUniswapV3Pool);

        address token0 = v3Pool.token0();
        address token1 = v3Pool.token1();

        bool zeroForOne = token0 == address(weth);

        address tokenOut = zeroForOne ? token1 : token0;

        console.log("token0: %s", token0);
        console.log("token1: %s", token1);
        console.log("zeroForOne: %s", zeroForOne);

        // simulate v3 swap
        IERC20 tokenOutContract = IERC20(tokenOut);

        uint256 balanceBefore = tokenOutContract.balanceOf(address(simulator));

        simulator.uniswapV3Swap(targetUniswapV3Pool, zeroForOne, amountIn);

        uint256 balanceAfter = tokenOutContract.balanceOf(address(simulator));

        console.log("balance before: %s", balanceBefore);
        console.log("balance after: %s", balanceAfter);

        assert(balanceAfter > balanceBefore);
    }

    function testFlashswapArbitrage() public {
        uint256 amountIn = 100000000000000000; // 0.1 ETH
        address targetUniswapV3Pool = 0xDeBead39628F93905dfc3E88003af40bf11189b0;

        IUniswapV3Pool v3Pool = IUniswapV3Pool(targetUniswapV3Pool);

        address token0 = v3Pool.token0();

        bool zeroForOne = token0 == address(weth);

        uint256 wethBalanceBefore = weth.balanceOf(address(simulator));

        simulator.flashswapLstArbitrage(
            targetUniswapV3Pool,
            zeroForOne,
            amountIn
        );

        uint256 wethBalanceAfter = weth.balanceOf(address(simulator));

        int256 profit = int256(wethBalanceAfter) - int256(wethBalanceBefore);
        console.log("profit: %s", profit);
    }

    function testBalancerV2Swap() public {
        // wrap ETH and send to simulator contract
        uint256 amountIn = 100000000000000000; // 0.1 ETH
        weth.deposit{value: amountIn}();

        weth.transfer(address(simulator), amountIn);

        // check that WETH has been received
        uint256 simulatorWethBalance = weth.balanceOf(address(simulator));
        console.log("simulator weth balance: %s", simulatorWethBalance);

        address balancerV2Vault = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;
        address targetBalancerV2Pool = 0x05b1a35FdBC43849aA836B00c8861696edce8cC4;

        IBalancerV2Vault vaultContract = IBalancerV2Vault(balancerV2Vault);

        IBalancerV2Pool v2Pool = IBalancerV2Pool(targetBalancerV2Pool);

        bytes32 poolId = v2Pool.getPoolId();

        console.log("poolId:");
        console.logBytes32(poolId);

        (IERC20[] memory tokens, uint256[] memory balances, ) = vaultContract
            .getPoolTokens(poolId);

        for (uint256 i = 0; i < tokens.length; i++) {
            console.log("Token %s: %s", i, address(tokens[i]));
            console.log("Balance %s: %s", i, balances[i]);
        }

        address token0 = address(tokens[0]);
        address token1 = address(tokens[1]);

        uint256 reserve0 = balances[0];
        uint256 reserve1 = balances[1];

        (address tokenIn, address tokenOut) = token0 == address(weth)
            ? (token0, token1)
            : (token1, token0);

        uint256 reserveIn = token0 == address(weth) ? reserve0 : reserve1;

        // simulate balancer v2 swap
        IERC20 tokenOutContract = IERC20(tokenOut);

        uint256 balanceBefore = tokenOutContract.balanceOf(address(simulator));

        uint256 swapAmountIn = 100000000000000; // 0.0001 ETH
        assert(reserveIn >= swapAmountIn);

        simulator.balancerV2Swap(poolId, tokenIn, tokenOut, swapAmountIn);

        uint256 balanceAfter = tokenOutContract.balanceOf(address(simulator));

        console.log("balance before: %s", balanceBefore);
        console.log("balance after: %s", balanceAfter);

        assert(balanceAfter > balanceBefore);
    }

    function testCurveV2Swap() public {
        // wrap ETH and send to simulator contract
        uint256 amountIn = 100000000000000000; // 0.1 ETH
        weth.deposit{value: amountIn}();

        weth.transfer(address(simulator), amountIn);

        // check that WETH has been received
        uint256 simulatorWethBalance = weth.balanceOf(address(simulator));
        console.log("simulator weth balance: %s", simulatorWethBalance);

        address targetCurveV2Pool = 0x429cCFCCa8ee06D2B41DAa6ee0e4F0EdBB77dFad;

        ICurveV2Pool v2Pool = ICurveV2Pool(targetCurveV2Pool);

        address token0 = v2Pool.coins(0);
        address token1 = v2Pool.coins(1);

        console.log("token0: %s", token0);
        console.log("token1: %s", token1);

        (uint256 tokenInIdx, uint256 tokenOutIdx) = token0 == address(weth)
            ? (0, 1)
            : (1, 0);

        address tokenOut = token0 == address(weth) ? token1 : token0;

        // simulate curve v2 swap
        IERC20 tokenOutContract = IERC20(tokenOut);

        uint256 balanceBefore = tokenOutContract.balanceOf(address(simulator));

        simulator.curveV2Swap(
            targetCurveV2Pool,
            tokenInIdx,
            tokenOutIdx,
            amountIn
        );

        uint256 balanceAfter = tokenOutContract.balanceOf(address(simulator));

        console.log("balance before: %s", balanceBefore);
        console.log("balance after: %s", balanceAfter);

        assert(balanceAfter > balanceBefore);
    }

    function testUniswapV3Arbitrage() public {
        address targetLst = 0x24Ae2dA0f361AA4BE46b48EB19C91e02c5e4f27E;
        address tokenIn = address(weth);
        uint256 amountIn = 100000000000000000; // 0.1 ETH

        address targetUniswapV3Pool = 0xDeBead39628F93905dfc3E88003af40bf11189b0;

        IUniswapV3Pool v3Pool = IUniswapV3Pool(targetUniswapV3Pool);
        address token0 = v3Pool.token0();
        bool zeroForOne = token0 == address(weth); // zeroForOne for buy, flip for sell

        bytes memory sellData = abi.encode(0, targetUniswapV3Pool, !zeroForOne);

        simulator.flashloanLstArbitrage(targetLst, tokenIn, amountIn, sellData);
    }

    function testBalancerV2Arbitrage() public {
        address targetLst = 0x24Ae2dA0f361AA4BE46b48EB19C91e02c5e4f27E;
        uint256 amountIn = 100000000000000; // 0.0001 ETH

        address balancerV2Vault = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;
        address targetBalancerV2Pool = 0x05b1a35FdBC43849aA836B00c8861696edce8cC4;

        IBalancerV2Vault vaultContract = IBalancerV2Vault(balancerV2Vault);
        IBalancerV2Pool v2Pool = IBalancerV2Pool(targetBalancerV2Pool);
        bytes32 poolId = v2Pool.getPoolId();

        (IERC20[] memory tokens, , ) = vaultContract.getPoolTokens(poolId);

        address token0 = address(tokens[0]);
        address token1 = address(tokens[1]);

        (address tokenIn, address tokenOut) = token0 == address(weth)
            ? (token0, token1)
            : (token1, token0);

        bytes memory sellData = abi.encode(1, poolId, tokenIn, tokenOut);

        IMevEth mevEth = IMevEth(targetLst);
        uint256 minDeposit = mevEth.MIN_DEPOSIT();

        if (amountIn > minDeposit) {
            simulator.flashloanLstArbitrage(
                targetLst,
                tokenIn,
                amountIn,
                sellData
            );
        } else {
            console.log("amountIn < minDeposit");
        }
    }

    function testCurveV2Arbitrage() public {
        address targetLst = 0x24Ae2dA0f361AA4BE46b48EB19C91e02c5e4f27E;
        address tokenIn = address(weth);
        uint256 amountIn = 100000000000000000; // 0.1 ETH

        address targetCurveV2Pool = 0x429cCFCCa8ee06D2B41DAa6ee0e4F0EdBB77dFad;
        ICurveV2Pool v2Pool = ICurveV2Pool(targetCurveV2Pool);

        address token0 = v2Pool.coins(0);

        (uint256 tokenInIdx, uint256 tokenOutIdx) = token0 == address(weth)
            ? (1, 0)
            : (0, 1); // flip because it's sell

        bytes memory sellData = abi.encode(
            2,
            targetCurveV2Pool,
            tokenInIdx,
            tokenOutIdx
        );

        simulator.flashloanLstArbitrage(targetLst, tokenIn, amountIn, sellData);
    }
}
