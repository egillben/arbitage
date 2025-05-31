// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./interfaces/IFlashLoanReceiver.sol";
import "./interfaces/ILendingPool.sol";
import "./interfaces/IUniswapV2Router.sol";
import "./interfaces/ICurveRouter.sol";
import "./interfaces/IERC20.sol";
import "./libraries/Ownable.sol";
import "./libraries/ReentrancyGuard.sol";
import "./libraries/SafeMath.sol";
import "./libraries/SlippageProtection.sol";

/**
 * @title ArbitrageExecutor
 * @dev Contract for executing arbitrage opportunities using flash loans
 */
contract ArbitrageExecutor is IFlashLoanReceiver, Ownable, ReentrancyGuard {
    using SafeMath for uint256;

    // Constants
    uint256 private constant BASIS_POINTS = 10000; // 100%
    uint256 private constant MAX_SLIPPAGE = 300; // 3% max slippage
    uint256 private constant MAX_PRICE_IMPACT = 500; // 5% max price impact
    
    // State variables
    address public lendingPoolAddress;
    address public uniswapRouterAddress;
    address public sushiswapRouterAddress;
    address public curveRouterAddress;
    
    // Authorized callers
    mapping(address => bool) public authorizedCallers;
    
    // Circuit breaker
    bool public emergencyStop;
    
    // Events
    event ArbitrageExecuted(
        address[] path,
        uint256 amountIn,
        uint256 amountOut,
        uint256 profit,
        string[] dexPath
    );
    
    event EmergencyStopActivated(address indexed activator);
    event EmergencyStopDeactivated(address indexed deactivator);
    event CallerAuthorized(address indexed caller);
    event CallerUnauthorized(address indexed caller);
    event TokensRecovered(address indexed token, uint256 amount);
    event ETHRecovered(uint256 amount);
    
    /**
     * @dev Constructor
     * @param _lendingPoolAddress Address of the Aave lending pool
     * @param _uniswapRouterAddress Address of the Uniswap V2 router
     * @param _sushiswapRouterAddress Address of the Sushiswap router
     * @param _curveRouterAddress Address of the Curve router
     */
    constructor(
        address _lendingPoolAddress,
        address _uniswapRouterAddress,
        address _sushiswapRouterAddress,
        address _curveRouterAddress
    ) {
        require(_lendingPoolAddress != address(0), "ArbitrageExecutor: lending pool address cannot be zero");
        require(_uniswapRouterAddress != address(0), "ArbitrageExecutor: uniswap router address cannot be zero");
        require(_sushiswapRouterAddress != address(0), "ArbitrageExecutor: sushiswap router address cannot be zero");
        require(_curveRouterAddress != address(0), "ArbitrageExecutor: curve router address cannot be zero");
        
        lendingPoolAddress = _lendingPoolAddress;
        uniswapRouterAddress = _uniswapRouterAddress;
        sushiswapRouterAddress = _sushiswapRouterAddress;
        curveRouterAddress = _curveRouterAddress;
        
        // Authorize the deployer
        authorizedCallers[msg.sender] = true;
        
        // Initialize circuit breaker
        emergencyStop = false;
    }
    
    /**
     * @dev Modifier to check if the caller is authorized
     */
    modifier onlyAuthorized() {
        require(authorizedCallers[msg.sender], "ArbitrageExecutor: caller is not authorized");
        _;
    }
    
    /**
     * @dev Modifier to check if the circuit breaker is not activated
     */
    modifier whenNotStopped() {
        require(!emergencyStop, "ArbitrageExecutor: emergency stop is active");
        _;
    }
    
    /**
     * @dev Authorize a caller
     * @param caller Address of the caller to authorize
     */
    function authorizeCaller(address caller) external onlyOwner {
        require(caller != address(0), "ArbitrageExecutor: caller address cannot be zero");
        authorizedCallers[caller] = true;
        emit CallerAuthorized(caller);
    }
    
    /**
     * @dev Unauthorize a caller
     * @param caller Address of the caller to unauthorize
     */
    function unauthorizeCaller(address caller) external onlyOwner {
        authorizedCallers[caller] = false;
        emit CallerUnauthorized(caller);
    }
    
    /**
     * @dev Activate the emergency stop
     */
    function activateEmergencyStop() external onlyOwner {
        emergencyStop = true;
        emit EmergencyStopActivated(msg.sender);
    }
    
    /**
     * @dev Deactivate the emergency stop
     */
    function deactivateEmergencyStop() external onlyOwner {
        emergencyStop = false;
        emit EmergencyStopDeactivated(msg.sender);
    }
    
    /**
     * @dev Recover ERC20 tokens sent to the contract by mistake
     * @param token Address of the token to recover
     * @param amount Amount of tokens to recover
     */
    function recoverERC20(address token, uint256 amount) external onlyOwner {
        IERC20(token).transfer(owner(), amount);
        emit TokensRecovered(token, amount);
    }
    
    /**
     * @dev Recover ETH sent to the contract by mistake
     */
    function recoverETH() external onlyOwner {
        uint256 balance = address(this).balance;
        require(balance > 0, "ArbitrageExecutor: no ETH to recover");
        
        (bool success, ) = owner().call{value: balance}("");
        require(success, "ArbitrageExecutor: ETH recovery failed");
        
        emit ETHRecovered(balance);
    }
    
    /**
     * @dev Execute a flash loan to perform arbitrage
     * @param assets The addresses of the assets to borrow
     * @param amounts The amounts of the assets to borrow
     * @param modes The modes of the flash loan (0 = no debt, 1 = stable, 2 = variable)
     * @param tokenPath The path of tokens to trade through
     * @param dexPath The path of DEXes to use for each trade
     * @param slippage The slippage tolerance in basis points
     */
    function executeArbitrage(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata modes,
        address[] calldata tokenPath,
        string[] calldata dexPath,
        uint256 slippage
    ) external onlyAuthorized whenNotStopped nonReentrant {
        require(assets.length == 1, "ArbitrageExecutor: only single asset flash loans supported");
        require(amounts.length == 1, "ArbitrageExecutor: only single amount flash loans supported");
        require(modes.length == 1, "ArbitrageExecutor: only single mode flash loans supported");
        require(tokenPath.length >= 2, "ArbitrageExecutor: token path must have at least 2 tokens");
        require(dexPath.length == tokenPath.length - 1, "ArbitrageExecutor: dex path length must be token path length - 1");
        require(slippage <= MAX_SLIPPAGE, "ArbitrageExecutor: slippage too high");
        
        // Encode the parameters for the flash loan
        bytes memory params = abi.encode(tokenPath, dexPath, slippage);
        
        // Execute the flash loan
        ILendingPool(lendingPoolAddress).flashLoan(
            address(this),
            assets,
            amounts,
            modes,
            address(this),
            params,
            0 // referral code
        );
    }
    
    /**
     * @dev Callback function for the flash loan
     * @param assets The addresses of the assets borrowed
     * @param amounts The amounts of the assets borrowed
     * @param premiums The premiums to pay for the flash loan
     * @param initiator The address that initiated the flash loan
     * @param params The encoded parameters for the arbitrage
     * @return A boolean indicating if the operation was successful
     */
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        // Ensure the caller is the lending pool
        require(msg.sender == lendingPoolAddress, "ArbitrageExecutor: caller is not lending pool");
        require(initiator == address(this), "ArbitrageExecutor: initiator is not this contract");
        
        // Decode the parameters
        (
            address[] memory tokenPath,
            string[] memory dexPath,
            uint256 slippage
        ) = abi.decode(params, (address[], string[], uint256));
        
        // Get the borrowed amount
        uint256 borrowedAmount = amounts[0];
        uint256 fee = premiums[0];
        uint256 totalToRepay = borrowedAmount.add(fee);
        
        // Execute the arbitrage
        uint256 finalAmount = executeArbitrageInternal(
            assets[0],
            borrowedAmount,
            tokenPath,
            dexPath,
            slippage
        );
        
        // Ensure we have enough to repay the loan
        require(finalAmount >= totalToRepay, "ArbitrageExecutor: insufficient funds to repay flash loan");
        
        // Calculate profit
        uint256 profit = finalAmount.sub(totalToRepay);
        
        // Approve the lending pool to take the repayment
        IERC20(assets[0]).approve(lendingPoolAddress, totalToRepay);
        
        // Emit event
        emit ArbitrageExecuted(
            tokenPath,
            borrowedAmount,
            finalAmount,
            profit,
            dexPath
        );
        
        return true;
    }
    
    /**
     * @dev Internal function to execute the arbitrage trades
     * @param initialToken The initial token of the arbitrage
     * @param initialAmount The initial amount of tokens
     * @param tokenPath The path of tokens to trade through
     * @param dexPath The path of DEXes to use for each trade
     * @param slippage The slippage tolerance in basis points
     * @return The final amount of tokens after all trades
     */
    function executeArbitrageInternal(
        address initialToken,
        uint256 initialAmount,
        address[] memory tokenPath,
        string[] memory dexPath,
        uint256 slippage
    ) internal returns (uint256) {
        require(tokenPath[0] == initialToken, "ArbitrageExecutor: initial token mismatch");
        
        uint256 currentAmount = initialAmount;
        
        // Execute each trade in the path
        for (uint256 i = 0; i < dexPath.length; i++) {
            address fromToken = tokenPath[i];
            address toToken = tokenPath[i + 1];
            
            // Approve the router to spend the tokens
            IERC20(fromToken).approve(getRouterAddress(dexPath[i]), currentAmount);
            
            // Execute the trade based on the DEX
            currentAmount = executeTrade(
                dexPath[i],
                fromToken,
                toToken,
                currentAmount,
                slippage
            );
        }
        
        // Ensure the final token is the same as the initial token
        require(tokenPath[tokenPath.length - 1] == initialToken, "ArbitrageExecutor: final token mismatch");
        
        return currentAmount;
    }
    
    /**
     * @dev Execute a trade on a specific DEX
     * @param dex The name of the DEX to use
     * @param fromToken The token to trade from
     * @param toToken The token to trade to
     * @param amount The amount of tokens to trade
     * @param slippage The slippage tolerance in basis points
     * @return The amount of tokens received
     */
    function executeTrade(
        string memory dex,
        address fromToken,
        address toToken,
        uint256 amount,
        uint256 slippage
    ) internal returns (uint256) {
        // Get the router address for the DEX
        address routerAddress = getRouterAddress(dex);
        
        // Calculate the minimum amount out based on slippage
        uint256 amountOutMin;
        
        if (keccak256(bytes(dex)) == keccak256(bytes("uniswap")) || 
            keccak256(bytes(dex)) == keccak256(bytes("sushiswap"))) {
            // For Uniswap and Sushiswap
            address[] memory path = new address[](2);
            path[0] = fromToken;
            path[1] = toToken;
            
            // Get the expected amount out
            uint256[] memory amountsOut = IUniswapV2Router(routerAddress).getAmountsOut(amount, path);
            uint256 expectedAmountOut = amountsOut[1];
            
            // Calculate minimum amount out with slippage
            amountOutMin = SlippageProtection.calculateMinimumAmountOut(expectedAmountOut, slippage);
            
            // Execute the swap
            uint256[] memory amounts = IUniswapV2Router(routerAddress).swapExactTokensForTokens(
                amount,
                amountOutMin,
                path,
                address(this),
                block.timestamp + 300 // 5 minutes deadline
            );
            
            return amounts[amounts.length - 1];
        } else if (keccak256(bytes(dex)) == keccak256(bytes("curve"))) {
            // For Curve
            (address bestPool, uint256 expectedAmountOut) = ICurveRouter(routerAddress).get_best_rate(
                fromToken,
                toToken,
                amount
            );
            
            // Calculate minimum amount out with slippage
            amountOutMin = SlippageProtection.calculateMinimumAmountOut(expectedAmountOut, slippage);
            
            // Prepare the route and swap parameters for Curve
            address[11] memory route;
            route[0] = fromToken;
            route[1] = bestPool;
            route[2] = toToken;
            
            uint256[5][5] memory swapParams;
            // Set swap parameters based on the pool type
            // This is a simplified version, in a real implementation
            // you would need to determine the correct indices for the tokens
            swapParams[0][0] = 0; // i (index of fromToken)
            swapParams[0][1] = 1; // j (index of toToken)
            swapParams[0][2] = 0; // swap type
            
            address[5] memory pools;
            pools[0] = bestPool;
            
            // Execute the swap
            uint256 received = ICurveRouter(routerAddress).exchange(
                route,
                swapParams,
                amount,
                amountOutMin,
                pools,
                address(this)
            );
            
            return received;
        } else {
            revert("ArbitrageExecutor: unsupported DEX");
        }
    }
    
    /**
     * @dev Get the router address for a specific DEX
     * @param dex The name of the DEX
     * @return The router address
     */
    function getRouterAddress(string memory dex) internal view returns (address) {
        if (keccak256(bytes(dex)) == keccak256(bytes("uniswap"))) {
            return uniswapRouterAddress;
        } else if (keccak256(bytes(dex)) == keccak256(bytes("sushiswap"))) {
            return sushiswapRouterAddress;
        } else if (keccak256(bytes(dex)) == keccak256(bytes("curve"))) {
            return curveRouterAddress;
        } else {
            revert("ArbitrageExecutor: unsupported DEX");
        }
    }
    
    /**
     * @dev Fallback function to receive ETH
     */
    fallback() external payable {}
}