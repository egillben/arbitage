// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title ICurveRouter
 * @dev Interface for the Curve Router
 */
interface ICurveRouter {
    /**
     * @notice Perform an exchange between two coins
     * @param _route Array of [initial token, pool, token, pool, token, ...]
     * @param _swap_params Array of [i, j, swap type] where i and j are the correct indices for the coins
     * @param _amount Amount of initial token to swap
     * @param _expected Minimum amount of final token to receive
     * @param _pools Array of pools for swaps via zap contracts
     * @param _receiver Address to transfer the final token to
     * @return Received amount of final token
     */
    function exchange(
        address[11] calldata _route,
        uint256[5][5] calldata _swap_params,
        uint256 _amount,
        uint256 _expected,
        address[5] calldata _pools,
        address _receiver
    ) external payable returns (uint256);

    /**
     * @notice Find the best route to exchange _from_token to _to_token
     * @param _from_token Initial token address
     * @param _to_token Final token address
     * @param _amount Amount of initial token to swap
     * @return Best pool address and expected amount out
     */
    function get_best_rate(
        address _from_token,
        address _to_token,
        uint256 _amount
    ) external view returns (address, uint256);

    /**
     * @notice Calculate the amount received when swapping between two coins
     * @param _from_token Initial token address
     * @param _to_token Final token address
     * @param _amount Amount of initial token to swap
     * @param _pools Array of pools to consider for the swap
     * @return Expected amount of final token received
     */
    function get_exchange_amount(
        address _from_token,
        address _to_token,
        uint256 _amount,
        address[8] calldata _pools
    ) external view returns (uint256);
}