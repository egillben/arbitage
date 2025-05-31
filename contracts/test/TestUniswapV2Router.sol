// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../interfaces/IERC20.sol";
import "./TestUniswapV2Factory.sol";
import "./TestUniswapV2Pair.sol";

contract TestUniswapV2Router {
    address public immutable factory;
    
    constructor(address _factory) {
        factory = _factory;
    }
    
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut) public pure returns (uint256 amountOut) {
        require(amountIn > 0, "TestUniswapV2Router: INSUFFICIENT_INPUT_AMOUNT");
        require(reserveIn > 0 && reserveOut > 0, "TestUniswapV2Router: INSUFFICIENT_LIQUIDITY");
        
        uint256 amountInWithFee = amountIn * 997; // 0.3% fee
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        amountOut = numerator / denominator;
    }
    
    function getAmountsOut(uint256 amountIn, address[] memory path) public view returns (uint256[] memory amounts) {
        require(path.length >= 2, "TestUniswapV2Router: INVALID_PATH");
        amounts = new uint256[](path.length);
        amounts[0] = amountIn;
        
        for (uint256 i; i < path.length - 1; i++) {
            (uint256 reserveIn, uint256 reserveOut) = getReserves(path[i], path[i + 1]);
            amounts[i + 1] = getAmountOut(amounts[i], reserveIn, reserveOut);
        }
    }
    
    function getReserves(address tokenA, address tokenB) internal view returns (uint256 reserveA, uint256 reserveB) {
        (address token0, address token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
        address pair = TestUniswapV2Factory(factory).getPair(token0, token1);
        require(pair != address(0), "TestUniswapV2Router: PAIR_NOT_FOUND");
        
        (uint256 reserve0, uint256 reserve1) = TestUniswapV2Pair(pair).getReserves();
        (reserveA, reserveB) = tokenA == token0 ? (reserve0, reserve1) : (reserve1, reserve0);
    }
    
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint256[] memory amounts) {
        require(deadline >= block.timestamp, "TestUniswapV2Router: EXPIRED");
        amounts = getAmountsOut(amountIn, path);
        require(amounts[amounts.length - 1] >= amountOutMin, "TestUniswapV2Router: INSUFFICIENT_OUTPUT_AMOUNT");
        
        address pair = TestUniswapV2Factory(factory).getPair(path[0], path[1]);
        require(pair != address(0), "TestUniswapV2Router: PAIR_NOT_FOUND");
        
        IERC20(path[0]).transferFrom(msg.sender, pair, amounts[0]);
        _swap(amounts, path, to);
        
        return amounts;
    }
    
    function _swap(uint256[] memory amounts, address[] memory path, address _to) internal {
        for (uint256 i; i < path.length - 1; i++) {
            (address input, address output) = (path[i], path[i + 1]);
            (address token0,) = input < output ? (input, output) : (output, input);
            uint256 amountOut = amounts[i + 1];
            (uint256 amount0Out, uint256 amount1Out) = input == token0 ? (uint256(0), amountOut) : (amountOut, uint256(0));
            address to = i < path.length - 2 ? TestUniswapV2Factory(factory).getPair(output, path[i + 2]) : _to;
            
            TestUniswapV2Pair(TestUniswapV2Factory(factory).getPair(input, output)).swap(
                amount0Out, amount1Out, to
            );
        }
    }
    
    function addLiquidity(
        address tokenA,
        address tokenB,
        uint256 amountADesired,
        uint256 amountBDesired,
        uint256 amountAMin,
        uint256 amountBMin,
        address to,
        uint256 deadline
    ) external returns (uint256 amountA, uint256 amountB, uint256 liquidity) {
        require(deadline >= block.timestamp, "TestUniswapV2Router: EXPIRED");
        
        address pair = _createPairIfNeeded(tokenA, tokenB);
        (amountA, amountB) = _calculateLiquidityAmounts(
            tokenA, tokenB, amountADesired, amountBDesired, amountAMin, amountBMin
        );
        
        IERC20(tokenA).transferFrom(msg.sender, pair, amountA);
        IERC20(tokenB).transferFrom(msg.sender, pair, amountB);
        liquidity = TestUniswapV2Pair(pair).mint(to);
    }
    
    function _createPairIfNeeded(address tokenA, address tokenB) internal returns (address pair) {
        pair = TestUniswapV2Factory(factory).getPair(tokenA, tokenB);
        if (pair == address(0)) {
            pair = TestUniswapV2Factory(factory).createPair(tokenA, tokenB);
        }
    }
    
    function _calculateLiquidityAmounts(
        address tokenA,
        address tokenB,
        uint256 amountADesired,
        uint256 amountBDesired,
        uint256 amountAMin,
        uint256 amountBMin
    ) internal view returns (uint256 amountA, uint256 amountB) {
        (uint256 reserveA, uint256 reserveB) = (0, 0);
        
        address pair = TestUniswapV2Factory(factory).getPair(tokenA, tokenB);
        if (pair != address(0)) {
            (reserveA, reserveB) = getReserves(tokenA, tokenB);
        }
        
        if (reserveA == 0 && reserveB == 0) {
            (amountA, amountB) = (amountADesired, amountBDesired);
        } else {
            uint256 amountBOptimal = (amountADesired * reserveB) / reserveA;
            if (amountBOptimal <= amountBDesired) {
                require(amountBOptimal >= amountBMin, "TestUniswapV2Router: INSUFFICIENT_B_AMOUNT");
                (amountA, amountB) = (amountADesired, amountBOptimal);
            } else {
                uint256 amountAOptimal = (amountBDesired * reserveA) / reserveB;
                require(amountAOptimal <= amountADesired, "TestUniswapV2Router: EXCESSIVE_A_AMOUNT");
                require(amountAOptimal >= amountAMin, "TestUniswapV2Router: INSUFFICIENT_A_AMOUNT");
                (amountA, amountB) = (amountAOptimal, amountBDesired);
            }
        }
    }
}