// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title SlippageProtection
 * @dev Library for handling slippage protection in DEX trades
 */
library SlippageProtection {
    /**
     * @dev Calculates the minimum amount out based on the expected amount and slippage tolerance
     * @param expectedAmount The expected amount out from the trade
     * @param slippageTolerance The slippage tolerance in basis points (1 = 0.01%)
     * @return The minimum amount out that should be received
     */
    function calculateMinimumAmountOut(
        uint256 expectedAmount,
        uint256 slippageTolerance
    ) internal pure returns (uint256) {
        // Ensure slippage tolerance is reasonable (max 10%)
        require(slippageTolerance <= 1000, "SlippageProtection: excessive slippage tolerance");
        
        // Calculate the minimum amount out
        // Formula: expectedAmount * (10000 - slippageTolerance) / 10000
        return (expectedAmount * (10000 - slippageTolerance)) / 10000;
    }

    /**
     * @dev Verifies that the actual amount received is not less than the minimum expected
     * @param actualAmount The actual amount received from the trade
     * @param minimumExpected The minimum expected amount
     * @return True if the actual amount is greater than or equal to the minimum expected
     */
    function verifyAmountReceived(
        uint256 actualAmount,
        uint256 minimumExpected
    ) internal pure returns (bool) {
        return actualAmount >= minimumExpected;
    }

    /**
     * @dev Calculates the price impact of a trade
     * @param inputAmount The input amount
     * @param outputAmount The output amount
     * @param inputPrice The price of the input token in a common unit (e.g., USD)
     * @param outputPrice The price of the output token in the same unit
     * @return The price impact in basis points (1 = 0.01%)
     */
    function calculatePriceImpact(
        uint256 inputAmount,
        uint256 outputAmount,
        uint256 inputPrice,
        uint256 outputPrice
    ) internal pure returns (uint256) {
        // Calculate the expected output value in terms of the input token
        uint256 inputValue = inputAmount * inputPrice;
        uint256 outputValue = outputAmount * outputPrice;
        
        // If the output value is greater than the input value, there's no negative price impact
        if (outputValue >= inputValue) {
            return 0;
        }
        
        // Calculate the price impact in basis points
        // Formula: (inputValue - outputValue) * 10000 / inputValue
        return ((inputValue - outputValue) * 10000) / inputValue;
    }

    /**
     * @dev Ensures that the price impact is not greater than the maximum allowed
     * @param priceImpact The calculated price impact in basis points
     * @param maxPriceImpact The maximum allowed price impact in basis points
     */
    function ensureMaxPriceImpact(
        uint256 priceImpact,
        uint256 maxPriceImpact
    ) internal pure {
        require(priceImpact <= maxPriceImpact, "SlippageProtection: price impact too high");
    }
}