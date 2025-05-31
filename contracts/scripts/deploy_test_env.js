// Deployment script for the test environment
// This script can be run directly with Node.js or with Hardhat
const fs = require("fs");
const path = require("path");

// Import the ethers library directly
const { ethers } = require("ethers");

// Set up a provider connected to the Hardhat node
const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");

// Hardhat's first account private key (from hardhat documentation)
const PRIVATE_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

// Load contract artifacts
const loadArtifact = (contractName) => {
  try {
    let artifactPath;
    
    // For ArbitrageExecutor, look in the main contracts directory
    if (contractName === "ArbitrageExecutor") {
      artifactPath = path.join(__dirname, `../../artifacts/contracts/${contractName}.sol/${contractName}.json`);
    } else {
      // For other contracts, look in the test directory
      artifactPath = path.join(__dirname, `../../artifacts/contracts/test/${contractName}.sol/${contractName}.json`);
    }
    
    console.log(`Looking for artifact at: ${artifactPath}`);
    return JSON.parse(fs.readFileSync(artifactPath, 'utf8'));
  } catch (error) {
    console.error(`Error loading artifact for ${contractName}:`, error.message);
    throw error;
  }
};

// Create a contract factory
const getContractFactory = (contractName) => {
  const artifact = loadArtifact(contractName);
  return new ethers.ContractFactory(artifact.abi, artifact.bytecode, wallet);
};

async function main() {
  console.log("Deploying test environment...");
  
  try {
    // Use the wallet as the deployer
    const deployer = wallet;
    console.log("Deploying with account:", deployer.address);
    
    // Deploy test tokens
    console.log("\nDeploying test tokens...");
    
    // Create contract factories
    const TestERC20Factory = getContractFactory("TestERC20");
    const TestUniswapV2FactoryFactory = getContractFactory("TestUniswapV2Factory");
    const TestUniswapV2RouterFactory = getContractFactory("TestUniswapV2Router");
    const TestLendingPoolFactory = getContractFactory("TestLendingPool");
    const ArbitrageExecutorFactory = getContractFactory("ArbitrageExecutor");
    
    // Deploy WETH with 18 decimals
    const weth = await TestERC20Factory.deploy(
      "Wrapped Ether",
      "WETH",
      18,
      ethers.utils.parseEther("10000") // 10,000 WETH
    );
    await weth.deployed();
    console.log("WETH deployed to:", weth.address);
    
    // Deploy USDC with 6 decimals
    const usdc = await TestERC20Factory.deploy(
      "USD Coin",
      "USDC",
      6,
      ethers.utils.parseUnits("10000000", 6) // 10,000,000 USDC
    );
    await usdc.deployed();
    console.log("USDC deployed to:", usdc.address);
    
    // Deploy DAI with 18 decimals
    const dai = await TestERC20Factory.deploy(
      "Dai Stablecoin",
      "DAI",
      18,
      ethers.utils.parseEther("10000000") // 10,000,000 DAI
    );
    await dai.deployed();
    console.log("DAI deployed to:", dai.address);
    
    // Deploy DEX contracts
    console.log("\nDeploying DEX contracts...");
    
    // Deploy Uniswap Factory
    const uniswapFactory = await TestUniswapV2FactoryFactory.deploy();
    await uniswapFactory.deployed();
    console.log("Uniswap Factory deployed to:", uniswapFactory.address);
    
    // Deploy Uniswap Router
    const uniswapRouter = await TestUniswapV2RouterFactory.deploy(uniswapFactory.address);
    await uniswapRouter.deployed();
    console.log("Uniswap Router deployed to:", uniswapRouter.address);
    
    // Deploy Sushiswap Factory (another instance of TestUniswapV2Factory)
    const sushiswapFactory = await TestUniswapV2FactoryFactory.deploy();
    await sushiswapFactory.deployed();
    console.log("Sushiswap Factory deployed to:", sushiswapFactory.address);
    
    // Deploy Sushiswap Router
    const sushiswapRouter = await TestUniswapV2RouterFactory.deploy(sushiswapFactory.address);
    await sushiswapRouter.deployed();
    console.log("Sushiswap Router deployed to:", sushiswapRouter.address);
    
    // For simplicity, we'll use the same contracts for Curve
    const curveFactory = await TestUniswapV2FactoryFactory.deploy();
    await curveFactory.deployed();
    console.log("Curve Factory deployed to:", curveFactory.address);
    
    const curveRouter = await TestUniswapV2RouterFactory.deploy(curveFactory.address);
    await curveRouter.deployed();
    console.log("Curve Router deployed to:", curveRouter.address);
    
    // Deploy Lending Pool
    console.log("\nDeploying Lending Pool...");
    const lendingPool = await TestLendingPoolFactory.deploy();
    await lendingPool.deployed();
    console.log("Lending Pool deployed to:", lendingPool.address);
    
    // Add liquidity to the lending pool
    console.log("\nAdding liquidity to the lending pool...");
    
    // Approve tokens for the lending pool
    await weth.approve(lendingPool.address, ethers.utils.parseEther("1000"));
    await usdc.approve(lendingPool.address, ethers.utils.parseUnits("1000000", 6));
    await dai.approve(lendingPool.address, ethers.utils.parseEther("1000000"));
    
    // Add liquidity
    await lendingPool.addLiquidity(weth.address, ethers.utils.parseEther("1000"));
    await lendingPool.addLiquidity(usdc.address, ethers.utils.parseUnits("1000000", 6));
    await lendingPool.addLiquidity(dai.address, ethers.utils.parseEther("1000000"));
    
    console.log("Added liquidity to the lending pool");
    
    // Create liquidity pools with price discrepancies
    console.log("\nCreating liquidity pools with price discrepancies...");
    
    // Approve tokens for the routers
    await weth.approve(uniswapRouter.address, ethers.utils.parseEther("1000"));
    await usdc.approve(uniswapRouter.address, ethers.utils.parseUnits("1000000", 6));
    await dai.approve(uniswapRouter.address, ethers.utils.parseEther("1000000"));
    
    await weth.approve(sushiswapRouter.address, ethers.utils.parseEther("1000"));
    await usdc.approve(sushiswapRouter.address, ethers.utils.parseUnits("1000000", 6));
    await dai.approve(sushiswapRouter.address, ethers.utils.parseEther("1000000"));
    
    // Create WETH-USDC pool on Uniswap with price 1 ETH = 2000 USDC
    await uniswapRouter.addLiquidity(
      weth.address,
      usdc.address,
      ethers.utils.parseEther("100"),
      ethers.utils.parseUnits("200000", 6), // 2000 USDC per ETH
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created WETH-USDC pool on Uniswap");
    
    // Create WETH-USDC pool on Sushiswap with price 1 ETH = 2020 USDC (1% higher)
    await sushiswapRouter.addLiquidity(
      weth.address,
      usdc.address,
      ethers.utils.parseEther("100"),
      ethers.utils.parseUnits("202000", 6), // 2020 USDC per ETH
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created WETH-USDC pool on Sushiswap");
    
    // Create WETH-DAI pool on Uniswap with price 1 ETH = 2000 DAI
    await uniswapRouter.addLiquidity(
      weth.address,
      dai.address,
      ethers.utils.parseEther("100"),
      ethers.utils.parseEther("200000"), // 2000 DAI per ETH
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created WETH-DAI pool on Uniswap");
    
    // Create WETH-DAI pool on Sushiswap with price 1 ETH = 1980 DAI (1% lower)
    await sushiswapRouter.addLiquidity(
      weth.address,
      dai.address,
      ethers.utils.parseEther("100"),
      ethers.utils.parseEther("198000"), // 1980 DAI per ETH
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created WETH-DAI pool on Sushiswap");
    
    // Create USDC-DAI pool on Uniswap with price 1 USDC = 0.99 DAI
    await uniswapRouter.addLiquidity(
      usdc.address,
      dai.address,
      ethers.utils.parseUnits("100000", 6),
      ethers.utils.parseEther("99000"), // 0.99 DAI per USDC
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created USDC-DAI pool on Uniswap");
    
    // Create USDC-DAI pool on Sushiswap with price 1 USDC = 1.01 DAI
    await sushiswapRouter.addLiquidity(
      usdc.address,
      dai.address,
      ethers.utils.parseUnits("100000", 6),
      ethers.utils.parseEther("101000"), // 1.01 DAI per USDC
      0,
      0,
      deployer.address,
      Math.floor(Date.now() / 1000) + 3600
    );
    console.log("Created USDC-DAI pool on Sushiswap");
    
    // Deploy ArbitrageExecutor
    console.log("\nDeploying ArbitrageExecutor...");
    const arbitrageExecutor = await ArbitrageExecutorFactory.deploy(
      lendingPool.address,
      uniswapRouter.address,
      sushiswapRouter.address,
      curveRouter.address
    );
    await arbitrageExecutor.deployed();
    console.log("ArbitrageExecutor deployed to:", arbitrageExecutor.address);
    
    // Get pool addresses
    console.log("\nRetrieving pool addresses...");
    
    // Get Uniswap pool addresses
    const uniswapWethUsdc = await uniswapFactory.getPair(weth.address, usdc.address);
    const uniswapWethDai = await uniswapFactory.getPair(weth.address, dai.address);
    const uniswapUsdcDai = await uniswapFactory.getPair(usdc.address, dai.address);
    
    console.log("Uniswap WETH-USDC pool:", uniswapWethUsdc);
    console.log("Uniswap WETH-DAI pool:", uniswapWethDai);
    console.log("Uniswap USDC-DAI pool:", uniswapUsdcDai);
    
    // Get Sushiswap pool addresses
    const sushiswapWethUsdc = await sushiswapFactory.getPair(weth.address, usdc.address);
    const sushiswapWethDai = await sushiswapFactory.getPair(weth.address, dai.address);
    const sushiswapUsdcDai = await sushiswapFactory.getPair(usdc.address, dai.address);
    
    console.log("Sushiswap WETH-USDC pool:", sushiswapWethUsdc);
    console.log("Sushiswap WETH-DAI pool:", sushiswapWethDai);
    console.log("Sushiswap USDC-DAI pool:", sushiswapUsdcDai);
    
    // Print summary
    console.log("\n=== Test Environment Deployed ===");
    console.log("WETH:", weth.address);
    console.log("USDC:", usdc.address);
    console.log("DAI:", dai.address);
    console.log("Uniswap Factory:", uniswapFactory.address);
    console.log("Uniswap Router:", uniswapRouter.address);
    console.log("Sushiswap Factory:", sushiswapFactory.address);
    console.log("Sushiswap Router:", sushiswapRouter.address);
    console.log("Curve Factory:", curveFactory.address);
    console.log("Curve Router:", curveRouter.address);
    console.log("Lending Pool:", lendingPool.address);
    console.log("ArbitrageExecutor:", arbitrageExecutor.address);
    
    // Create the config object in the format expected by the bot
    const configJson = {
      "ethereum": {
        "rpc_url": "http://localhost:8545",
        "chain_id": 31337,
        "wallet_address": deployer.address,
        "max_block_lookback": 10,
        "ws_timeout_seconds": 30,
        "use_websocket": false,
        "polling_interval_ms": 1000
      },
      "mev_share": {
        "api_url": "https://mev-share.flashbots.net",
        "enabled": false,
        "max_validator_tip": 2
      },
      "flash_loan": {
        "aave_lending_pool": lendingPool.address,
        "max_borrow_amount": 100.0,
        "tokens": [
          {
            "symbol": "WETH",
            "address": weth.address,
            "decimals": 18
          },
          {
            "symbol": "USDC",
            "address": usdc.address,
            "decimals": 6
          },
          {
            "symbol": "DAI",
            "address": dai.address,
            "decimals": 18
          }
        ]
      },
      "dex": {
        "uniswap": {
          "enabled": true,
          "factory_address": uniswapFactory.address,
          "router_address": uniswapRouter.address,
          "pools": [
            {
              "name": "WETH-USDC",
              "address": uniswapWethUsdc,
              "token0": weth.address,
              "token1": usdc.address
            },
            {
              "name": "WETH-DAI",
              "address": uniswapWethDai,
              "token0": weth.address,
              "token1": dai.address
            },
            {
              "name": "USDC-DAI",
              "address": uniswapUsdcDai,
              "token0": usdc.address,
              "token1": dai.address
            }
          ]
        },
        "sushiswap": {
          "enabled": true,
          "factory_address": sushiswapFactory.address,
          "router_address": sushiswapRouter.address,
          "pools": [
            {
              "name": "WETH-USDC",
              "address": sushiswapWethUsdc,
              "token0": weth.address,
              "token1": usdc.address
            },
            {
              "name": "WETH-DAI",
              "address": sushiswapWethDai,
              "token0": weth.address,
              "token1": dai.address
            },
            {
              "name": "USDC-DAI",
              "address": sushiswapUsdcDai,
              "token0": usdc.address,
              "token1": dai.address
            }
          ]
        },
        "curve": {
          "enabled": true,
          "factory_address": curveFactory.address,
          "router_address": curveRouter.address,
          "pools": []
        }
      },
      "arbitrage": {
        "min_profit_threshold": 0.01,
        "max_hops": 3,
        "slippage_tolerance": 0.5,
        "evaluation_timeout_ms": 500,
        "max_concurrent_evaluations": 5,
        "contract": {
          "contract_address": arbitrageExecutor.address,
          "deploy_if_missing": false,
          "deployment_gas_limit": 5000000
        }
      },
      "gas": {
        "strategy": "fixed",
        "max_gas_price": 10,
        "base_fee_multiplier": 1.0,
        "priority_fee": 1,
        "gas_limit": 500000
      },
      "security": {
        "transaction_timeout": 60,
        "min_price_sources": 1,
        "max_price_deviation": 5.0,
        "simulate_transactions": true,
        "max_execution_slippage": 2.0
      },
      "test_mode": true
    };
    
    // Save the addresses to the correct file for the bot
    const configPath = path.join(__dirname, '../../mev_arbitrage_bot/config.test_contracts.json');
    fs.writeFileSync(configPath, JSON.stringify(configJson, null, 2));
    console.log(`\nConfig saved to ${configPath}`);
    
    // Also save a copy to test-addresses.json for reference
    fs.writeFileSync("test-addresses.json", JSON.stringify({
      weth: weth.address,
      usdc: usdc.address,
      dai: dai.address,
      uniswapFactory: uniswapFactory.address,
      uniswapRouter: uniswapRouter.address,
      sushiswapFactory: sushiswapFactory.address,
      sushiswapRouter: sushiswapRouter.address,
      curveFactory: curveFactory.address,
      curveRouter: curveRouter.address,
      lendingPool: lendingPool.address,
      arbitrageExecutor: arbitrageExecutor.address,
      uniswapWethUsdc: uniswapWethUsdc,
      uniswapWethDai: uniswapWethDai,
      uniswapUsdcDai: uniswapUsdcDai,
      sushiswapWethUsdc: sushiswapWethUsdc,
      sushiswapWethDai: sushiswapWethDai,
      sushiswapUsdcDai: sushiswapUsdcDai
    }, null, 2));
    console.log("Addresses also saved to test-addresses.json");
  } catch (error) {
    console.error("Error in deployment:", error.message || error);
    process.exit(1);
  }
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("Deployment failed:", error.message || error);
    console.error("Make sure the Hardhat node is running:");
    console.error("npx hardhat node");
    process.exit(1);
  });