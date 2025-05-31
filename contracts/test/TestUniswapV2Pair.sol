// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../interfaces/IERC20.sol";

contract TestUniswapV2Pair {
    address public token0;
    address public token1;
    
    uint256 private reserve0;
    uint256 private reserve1;
    
    uint256 private constant MINIMUM_LIQUIDITY = 10**3;
    uint256 private totalSupply;
    mapping(address => uint256) private balances;
    
    event Mint(address indexed sender, uint256 amount0, uint256 amount1);
    event Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to);
    event Swap(
        address indexed sender,
        uint256 amount0In,
        uint256 amount1In,
        uint256 amount0Out,
        uint256 amount1Out,
        address indexed to
    );
    event Sync(uint256 reserve0, uint256 reserve1);
    
    function initialize(address _token0, address _token1) external {
        require(token0 == address(0) && token1 == address(0), "TestUniswapV2Pair: ALREADY_INITIALIZED");
        token0 = _token0;
        token1 = _token1;
    }
    
    function getReserves() public view returns (uint256 _reserve0, uint256 _reserve1) {
        _reserve0 = reserve0;
        _reserve1 = reserve1;
    }
    
    function mint(address to) external returns (uint256 liquidity) {
        (uint256 _reserve0, uint256 _reserve1) = getReserves();
        uint256 balance0 = IERC20(token0).balanceOf(address(this));
        uint256 balance1 = IERC20(token1).balanceOf(address(this));
        uint256 amount0 = balance0 - _reserve0;
        uint256 amount1 = balance1 - _reserve1;
        
        if (totalSupply == 0) {
            liquidity = Math.sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY;
            balances[address(0)] = MINIMUM_LIQUIDITY; // Permanently lock the first MINIMUM_LIQUIDITY tokens
        } else {
            liquidity = Math.min(
                (amount0 * totalSupply) / _reserve0,
                (amount1 * totalSupply) / _reserve1
            );
        }
        
        require(liquidity > 0, "TestUniswapV2Pair: INSUFFICIENT_LIQUIDITY_MINTED");
        balances[to] += liquidity;
        totalSupply += liquidity;
        
        _update(balance0, balance1);
        emit Mint(msg.sender, amount0, amount1);
    }
    
    function burn(address to) external returns (uint256 amount0, uint256 amount1) {
        uint256 balance0 = IERC20(token0).balanceOf(address(this));
        uint256 balance1 = IERC20(token1).balanceOf(address(this));
        uint256 liquidity = balances[address(this)];
        
        amount0 = (liquidity * balance0) / totalSupply;
        amount1 = (liquidity * balance1) / totalSupply;
        require(amount0 > 0 && amount1 > 0, "TestUniswapV2Pair: INSUFFICIENT_LIQUIDITY_BURNED");
        
        balances[address(this)] = 0;
        totalSupply -= liquidity;
        
        IERC20(token0).transfer(to, amount0);
        IERC20(token1).transfer(to, amount1);
        
        balance0 = IERC20(token0).balanceOf(address(this));
        balance1 = IERC20(token1).balanceOf(address(this));
        
        _update(balance0, balance1);
        emit Burn(msg.sender, amount0, amount1, to);
    }
    
    function swap(uint256 amount0Out, uint256 amount1Out, address to) external {
        require(amount0Out > 0 || amount1Out > 0, "TestUniswapV2Pair: INSUFFICIENT_OUTPUT_AMOUNT");
        (uint256 _reserve0, uint256 _reserve1) = getReserves();
        require(amount0Out < _reserve0 && amount1Out < _reserve1, "TestUniswapV2Pair: INSUFFICIENT_LIQUIDITY");
        
        uint256 balance0;
        uint256 balance1;
        {
            address _token0 = token0;
            address _token1 = token1;
            require(to != _token0 && to != _token1, "TestUniswapV2Pair: INVALID_TO");
            
            if (amount0Out > 0) IERC20(_token0).transfer(to, amount0Out);
            if (amount1Out > 0) IERC20(_token1).transfer(to, amount1Out);
            
            balance0 = IERC20(_token0).balanceOf(address(this));
            balance1 = IERC20(_token1).balanceOf(address(this));
        }
        
        uint256 amount0In = balance0 > _reserve0 - amount0Out ? balance0 - (_reserve0 - amount0Out) : 0;
        uint256 amount1In = balance1 > _reserve1 - amount1Out ? balance1 - (_reserve1 - amount1Out) : 0;
        require(amount0In > 0 || amount1In > 0, "TestUniswapV2Pair: INSUFFICIENT_INPUT_AMOUNT");
        
        // Verify k = x * y is preserved
        {
            uint256 balance0Adjusted = balance0 * 1000 - amount0In * 3;
            uint256 balance1Adjusted = balance1 * 1000 - amount1In * 3;
            require(
                balance0Adjusted * balance1Adjusted >= _reserve0 * _reserve1 * 1000**2,
                "TestUniswapV2Pair: K"
            );
        }
        
        _update(balance0, balance1);
        emit Swap(msg.sender, amount0In, amount1In, amount0Out, amount1Out, to);
    }
    
    function _update(uint256 balance0, uint256 balance1) private {
        reserve0 = balance0;
        reserve1 = balance1;
        emit Sync(reserve0, reserve1);
    }
}

// Simple Math library for sqrt function
library Math {
    function min(uint256 x, uint256 y) internal pure returns (uint256 z) {
        z = x < y ? x : y;
    }
    
    function sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }
}