// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./TestUniswapV2Pair.sol";

contract TestUniswapV2Factory {
    mapping(address => mapping(address => address)) public getPair;
    address[] public allPairs;
    
    event PairCreated(address indexed token0, address indexed token1, address pair, uint);
    
    function createPair(address tokenA, address tokenB) external returns (address pair) {
        require(tokenA != tokenB, 'TestUniswapV2Factory: IDENTICAL_ADDRESSES');
        (address token0, address token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
        require(token0 != address(0), 'TestUniswapV2Factory: ZERO_ADDRESS');
        require(getPair[token0][token1] == address(0), 'TestUniswapV2Factory: PAIR_EXISTS');
        
        TestUniswapV2Pair newPair = new TestUniswapV2Pair();
        newPair.initialize(token0, token1);
        
        pair = address(newPair);
        getPair[token0][token1] = pair;
        getPair[token1][token0] = pair;
        allPairs.push(pair);
        
        emit PairCreated(token0, token1, pair, allPairs.length);
    }
    
    function allPairsLength() external view returns (uint) {
        return allPairs.length;
    }
}