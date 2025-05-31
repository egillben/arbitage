// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title ILendingPool
 * @dev Interface for the Aave Lending Pool
 */
interface ILendingPool {
    /**
     * @notice Allows smart contracts to access the liquidity of the pool within one transaction,
     * as long as the amount taken plus a fee is returned.
     * @dev IMPORTANT There are security concerns for developers of flashloan receiver contracts
     * that must be kept into consideration. For further details please visit
     * https://docs.aave.com/developers/
     * @param receiverAddress The address of the contract receiving the funds, implementing IFlashLoanReceiver interface
     * @param assets The addresses of the assets being flash-borrowed
     * @param amounts The amounts of the assets being flash-borrowed
     * @param modes Types of the debt to open if the flash loan is not returned:
     *   0 -> Don't open any debt, just revert if funds can't be transferred from the receiver
     *   1 -> Open debt at stable rate for the value of the amount flash-borrowed to the `onBehalfOf` address
     *   2 -> Open debt at variable rate for the value of the amount flash-borrowed to the `onBehalfOf` address
     * @param onBehalfOf The address that will receive the debt in the case of using on `modes` 1 or 2
     * @param params Variadic packed params to pass to the receiver as extra information
     * @param referralCode Code used to register the integrator originating the operation, for potential rewards.
     *   0 if the action is executed directly by the user, without any middle-man
     */
    function flashLoan(
        address receiverAddress,
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata modes,
        address onBehalfOf,
        bytes calldata params,
        uint16 referralCode
    ) external;

    /**
     * @notice Returns the user account data across all the reserves
     * @param user The address of the user
     * @return totalCollateralETH The total collateral in ETH of the user
     * @return totalDebtETH The total debt in ETH of the user
     * @return availableBorrowsETH The borrowing power left of the user
     * @return currentLiquidationThreshold The liquidation threshold of the user
     * @return ltv The loan to value of the user
     * @return healthFactor The current health factor of the user
     */
    function getUserAccountData(
        address user
    )
        external
        view
        returns (
            uint256 totalCollateralETH,
            uint256 totalDebtETH,
            uint256 availableBorrowsETH,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        );

    /**
     * @notice Returns the normalized income per unit of asset
     * @param asset The address of the underlying asset of the reserve
     * @return The normalized income
     */
    function getReserveNormalizedIncome(
        address asset
    ) external view returns (uint256);

    /**
     * @notice Returns the list of the initialized reserves
     * @return The addresses of the reserves
     */
    function getReservesList() external view returns (address[] memory);
}