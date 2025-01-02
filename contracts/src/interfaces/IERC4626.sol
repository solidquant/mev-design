// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC4626 {
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
