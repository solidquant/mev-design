// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.20;

interface ICurveV2Pool {
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
