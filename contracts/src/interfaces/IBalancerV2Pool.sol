// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.20;

interface IBalancerV2Pool {
    function getPoolId() external view returns (bytes32);
}
