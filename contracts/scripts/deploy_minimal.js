// Minimal deployment script for test environment
const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Deploying minimal test environment...");
  
  // Connect to the local hardhat node
  const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");
  
  // Get the first account as the deployer
  const accounts = await provider.listAccounts();
  const deployer = provider.getSigner(accounts[0]);
  const deployerAddress = await deployer.getAddress();
  console.log("Deploying with account:", deployerAddress);
  
  // Get contract factories
  const TestERC20 = require("../artifacts/contracts/test/TestERC20.sol/TestERC20.json");
  const TestUniswapV2Factory = require("../artifacts/contracts/test/TestUniswapV2Factory.sol/TestUniswapV2Factory.json");
  const TestUniswapV2Router = require("../artifacts/contracts/test/TestUniswapV2Router.sol/TestUniswapV2Router.json");
  const TestLendingPool = require("../artifacts/contracts/test/TestLendingPool.sol/TestLendingPool.json");
  const ArbitrageExecutor = require("../artifacts/contracts/ArbitrageExecutor.sol/ArbitrageExecutor.json");
  
  // Deploy WETH
  console.log("\nDeploying WETH...");
  const wethFactory = new ethers.ContractFactory(TestERC20.abi, TestERC20.bytecode, deployer);
  const weth = await wethFactory.deploy(
    "Wrapped Ether",
    "WETH",
    18,
    ethers.utils.parseEther("10000") // 10,000 WETH
  );
  await weth.deployed();
  console.log("WETH deployed to:", weth.address);
  
  // Deploy USDC
  console.log("\nDeploying USDC...");
  const usdcFactory = new ethers.ContractFactory(TestERC20.abi, TestERC20.bytecode, deployer);
  const usdc = await usdcFactory.deploy(
    "USD Coin",
    "USDC",
    6,
    ethers.utils.parseUnits("10000000", 6) // 10,000,000 USDC
  );
  await usdc.deployed();
  console.log("USDC deployed to:", usdc.address);
  
  // Deploy DAI
  console.log("\nDeploying DAI...");
  const daiFactory = new ethers.ContractFactory(TestERC20.abi, TestERC20.bytecode, deployer);
  const dai = await daiFactory.deploy(
    "Dai Stablecoin",
    "DAI",
    18,
    ethers.utils.parseEther("10000000") // 10,000,000 DAI
  );
  await dai.deployed();
  console.log("DAI deployed to:", dai.address);
  
  // Deploy Uniswap Factory
  console.log("\nDeploying Uniswap Factory...");
  const uniswapFactoryFactory = new ethers.ContractFactory(TestUniswapV2Factory.abi, TestUniswapV2Factory.bytecode, deployer);
  const uniswapFactory = await uniswapFactoryFactory.deploy();
  await uniswapFactory.deployed();
  console.log("Uniswap Factory deployed to:", uniswapFactory.address);
  
  // Deploy Uniswap Router
  console.log("\nDeploying Uniswap Router...");
  const uniswapRouterFactory = new ethers.ContractFactory(TestUniswapV2Router.abi, TestUniswapV2Router.bytecode, deployer);
  const uniswapRouter = await uniswapRouterFactory.deploy(uniswapFactory.address);
  await uniswapRouter.deployed();
  console.log("Uniswap Router deployed to:", uniswapRouter.address);
  
  // Deploy Sushiswap Factory
  console.log("\nDeploying Sushiswap Factory...");
  const sushiswapFactoryFactory = new ethers.ContractFactory(TestUniswapV2Factory.abi, TestUniswapV2Factory.bytecode, deployer);
  const sushiswapFactory = await sushiswapFactoryFactory.deploy();
  await sushiswapFactory.deployed();
  console.log("Sushiswap Factory deployed to:", sushiswapFactory.address);
  
  // Deploy Sushiswap Router
  console.log("\nDeploying Sushiswap Router...");
  const sushiswapRouterFactory = new ethers.ContractFactory(TestUniswapV2Router.abi, TestUniswapV2Router.bytecode, deployer);
  const sushiswapRouter = await sushiswapRouterFactory.deploy(sushiswapFactory.address);
  await sushiswapRouter.deployed();
  console.log("Sushiswap Router deployed to:", sushiswapRouter.address);
  
  // Deploy Curve Factory (using the same contract as Uniswap for simplicity)
  console.log("\nDeploying Curve Factory...");
  const curveFactoryFactory = new ethers.ContractFactory(TestUniswapV2Factory.abi, TestUniswapV2Factory.bytecode, deployer);
  const curveFactory = await curveFactoryFactory.deploy();
  await curveFactory.deployed();
  console.log("Curve Factory deployed to:", curveFactory.address);
  
  // Deploy Curve Router
  console.log("\nDeploying Curve Router...");
  const curveRouterFactory = new ethers.ContractFactory(TestUniswapV2Router.abi, TestUniswapV2Router.bytecode, deployer);
  const curveRouter = await curveRouterFactory.deploy(curveFactory.address);
  await curveRouter.deployed();
  console.log("Curve Router deployed to:", curveRouter.address);
  
  // Deploy Lending Pool
  console.log("\nDeploying Lending Pool...");
  const lendingPoolFactory = new ethers.ContractFactory(TestLendingPool.abi, TestLendingPool.bytecode, deployer);
  const lendingPool = await lendingPoolFactory.deploy();
  await lendingPool.deployed();
  console.log("Lending Pool deployed to:", lendingPool.address);
  
  // Deploy ArbitrageExecutor
  console.log("\nDeploying ArbitrageExecutor...");
  const arbitrageExecutorFactory = new ethers.ContractFactory(ArbitrageExecutor.abi, ArbitrageExecutor.bytecode, deployer);
  const arbitrageExecutor = await arbitrageExecutorFactory.deploy(
    lendingPool.address,
    uniswapRouter.address,
    sushiswapRouter.address,
    curveRouter.address
  );
  await arbitrageExecutor.deployed();
  console.log("ArbitrageExecutor deployed to:", arbitrageExecutor.address);
  
  // Create the config object in the format expected by the bot
  const configJson = {
    "ethereum": {
      "rpc_url": "http://localhost:8545",
      "chain_id": 31337,
      "wallet_address": deployerAddress,
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
        "pools": []
      },
      "sushiswap": {
        "enabled": true,
        "factory_address": sushiswapFactory.address,
        "router_address": sushiswapRouter.address,
        "pools": []
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
    }
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
    arbitrageExecutor: arbitrageExecutor.address
  }, null, 2));
  console.log("Addresses also saved to test-addresses.json");
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });