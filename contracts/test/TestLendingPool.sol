// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../interfaces/IERC20.sol";
import "../interfaces/IFlashLoanReceiver.sol";
import "../interfaces/ILendingPool.sol";

/**
 * @title TestLendingPool
 * @dev A simplified Aave lending pool for testing flash loans
 */
contract TestLendingPool is ILendingPool {
    // Fee for flash loans (0.09%)
    uint256 public constant FLASH_LOAN_FEE = 9;
    uint256 public constant FLASH_LOAN_FEE_PRECISION = 10000;
    
    // Mapping of token balances
    mapping(address => uint256) public reserves;
    
    // List of initialized reserves
    address[] private _reservesList;
    
    // Mapping to track if a reserve is initialized
    mapping(address => bool) private _reservesInitialized;
    
    /**
     * @dev Add liquidity to the lending pool
     * @param asset The asset to add liquidity for
     * @param amount The amount to add
     */
    function addLiquidity(address asset, uint256 amount) external {
        IERC20(asset).transferFrom(msg.sender, address(this), amount);
        reserves[asset] += amount;
        
        // Initialize the reserve if not already
        if (!_reservesInitialized[asset]) {
            _reservesList.push(asset);
            _reservesInitialized[asset] = true;
        }
    }
    
    /**
     * @dev Execute a flash loan
     * @param receiverAddress The address of the contract receiving the funds
     * @param assets The addresses of the assets being flash-borrowed
     * @param amounts The amounts of the assets being flash-borrowed
     * @param modes The modes of the flash loan (0 = no debt, 1 = stable, 2 = variable)
     * @param onBehalfOf The address that will receive the debt in the case of using credit delegation
     * @param params Variadic packed params to pass to the receiver as extra information
     * @param referralCode Code used to register the integrator originating the operation
     */
    function flashLoan(
        address receiverAddress,
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata modes,
        address onBehalfOf,
        bytes calldata params,
        uint16 referralCode
    ) external override {
        require(assets.length == amounts.length, "TestLendingPool: INCONSISTENT_PARAMS");
        require(assets.length == modes.length, "TestLendingPool: INCONSISTENT_PARAMS");
        
        uint256[] memory premiums = new uint256[](assets.length);
        
        // Calculate premiums
        for (uint256 i = 0; i < assets.length; i++) {
            premiums[i] = (amounts[i] * FLASH_LOAN_FEE) / FLASH_LOAN_FEE_PRECISION;
            
            // Transfer the asset to the receiver
            require(reserves[assets[i]] >= amounts[i], "TestLendingPool: INSUFFICIENT_LIQUIDITY");
            IERC20(assets[i]).transfer(receiverAddress, amounts[i]);
        }
        
        // Execute the flash loan callback
        require(
            IFlashLoanReceiver(receiverAddress).executeOperation(
                assets,
                amounts,
                premiums,
                msg.sender,
                params
            ),
            "TestLendingPool: INVALID_FLASH_LOAN_EXECUTOR_RETURN"
        );
        
        // Repay the flash loan with premium
        for (uint256 i = 0; i < assets.length; i++) {
            uint256 amountOwed = amounts[i] + premiums[i];
            IERC20(assets[i]).transferFrom(receiverAddress, address(this), amountOwed);
            reserves[assets[i]] += premiums[i]; // Add the premium to the reserves
        }
    }
    
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
        override
        returns (
            uint256 totalCollateralETH,
            uint256 totalDebtETH,
            uint256 availableBorrowsETH,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        )
    {
        // For testing purposes, we return dummy values
        return (1000 ether, 0, 1000 ether, 8000, 8000, type(uint256).max);
    }
    
    /**
     * @notice Returns the normalized income per unit of asset
     * @param asset The address of the underlying asset of the reserve
     * @return The normalized income
     */
    function getReserveNormalizedIncome(
        address asset
    ) external view override returns (uint256) {
        // For testing purposes, we return a constant value
        return 10**27; // RAY = 10^27
    }
    
    /**
     * @notice Returns the list of the initialized reserves
     * @return The addresses of the reserves
     */
    function getReservesList() external view override returns (address[] memory) {
        return _reservesList;
    }
}