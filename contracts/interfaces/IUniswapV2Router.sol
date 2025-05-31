// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title IUniswapV2Router
 * @dev Interface for the Uniswap V2 Router
 */
interface IUniswapV2Router {
    /**
     * @notice Swaps an exact amount of input tokens for as many output tokens as possible
     * @param amountIn The amount of input tokens to send
     * @param amountOutMin The minimum amount of output tokens that must be received
     * @param path An array of token addresses. path.length must be >= 2.
     * @param to The address to receive the output tokens
     * @param deadline Unix timestamp after which the transaction will revert
     * @return amounts The input token amount and all subsequent output token amounts
     */
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);

    /**
     * @notice Swaps an exact amount of ETH for as many output tokens as possible
     * @param amountOutMin The minimum amount of output tokens that must be received
     * @param path An array of token addresses. path[0] must be WETH
     * @param to The address to receive the output tokens
     * @param deadline Unix timestamp after which the transaction will revert
     * @return amounts The input token amount and all subsequent output token amounts
     */
    function swapExactETHForTokens(
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external payable returns (uint[] memory amounts);

    /**
     * @notice Swaps an exact amount of tokens for as much ETH as possible
     * @param amountIn The amount of input tokens to send
     * @param amountOutMin The minimum amount of output ETH that must be received
     * @param path An array of token addresses. path[path.length-1] must be WETH
     * @param to The address to receive the output ETH
     * @param deadline Unix timestamp after which the transaction will revert
     * @return amounts The input token amount and all subsequent output token amounts
     */
    function swapExactTokensForETH(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);

    /**
     * @notice Given an input amount of an asset and pair reserves, returns the maximum output amount of the other asset
     * @param amountIn The input amount of the asset
     * @param path An array of token addresses
     * @return amounts The input token amount and all subsequent output token amounts
     */
    function getAmountsOut(
        uint amountIn,
        address[] calldata path
    ) external view returns (uint[] memory amounts);
}