// Deployment script using hardhat-ethers plugin
const fs = require("fs");
const path = require("path");

// This script must be run with hardhat
async function main() {
  console.log("Deploying test environment...");
  
  // Get the network
  const network = await ethers.provider.getNetwork();
  console.log(`Connected to network: ${network.name} (${network.chainId})`);
  
  // Get signers
  const [deployer] = await ethers.getSigners();
  console.log("Deploying with account:", deployer.address);
  
  // Deploy test tokens
  console.log("\nDeploying test tokens...");
  
  const TestERC20 = await ethers.getContractFactory("TestERC20");
  
  // Deploy WETH with 18 decimals
  const weth = await TestERC20.deploy(
    "Wrapped Ether",
    "WETH",
    18,
    ethers.utils.parseEther("10000") // 10,000 WETH
  );
  await weth.deployed();
  console.log("WETH deployed to:", weth.address);
  
  // Deploy USDC with 6 decimals
  const usdc = await TestERC20.deploy(
    "USD Coin",
    "USDC",
    6,
    ethers.utils.parseUnits("10000000", 6) // 10,000,000 USDC
  );
  await usdc.deployed();
  console.log("USDC deployed to:", usdc.address);
  
  // Deploy DAI with 18 decimals
  const dai = await TestERC20.deploy(
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
  const TestUniswapV2Factory = await ethers.getContractFactory("TestUniswapV2Factory");
  const uniswapFactory = await TestUniswapV2Factory.deploy();
  await uniswapFactory.deployed();
  console.log("Uniswap Factory deployed to:", uniswapFactory.address);
  
  // Deploy Uniswap Router
  const TestUniswapV2Router = await ethers.getContractFactory("TestUniswapV2Router");
  const uniswapRouter = await TestUniswapV2Router.deploy(uniswapFactory.address);
  await uniswapRouter.deployed();
  console.log("Uniswap Router deployed to:", uniswapRouter.address);
  
  // Deploy Sushiswap Factory (another instance of TestUniswapV2Factory)
  const sushiswapFactory = await TestUniswapV2Factory.deploy();
  await sushiswapFactory.deployed();
  console.log("Sushiswap Factory deployed to:", sushiswapFactory.address);
  
  // Deploy Sushiswap Router
  const sushiswapRouter = await TestUniswapV2Router.deploy(sushiswapFactory.address);
  await sushiswapRouter.deployed();
  console.log("Sushiswap Router deployed to:", sushiswapRouter.address);
  
  // For simplicity, we'll use the same contracts for Curve
  const curveFactory = await TestUniswapV2Factory.deploy();
  await curveFactory.deployed();
  console.log("Curve Factory deployed to:", curveFactory.address);
  
  const curveRouter = await TestUniswapV2Router.deploy(curveFactory.address);
  await curveRouter.deployed();
  console.log("Curve Router deployed to:", curveRouter.address);
  
  // Deploy Lending Pool
  console.log("\nDeploying Lending Pool...");
  const TestLendingPool = await ethers.getContractFactory("TestLendingPool");
  const lendingPool = await TestLendingPool.deploy();
  await lendingPool.deployed();
  console.log("Lending Pool deployed to:", lendingPool.address);
  
  // Deploy ArbitrageExecutor
  console.log("\nDeploying ArbitrageExecutor...");
  const ArbitrageExecutor = await ethers.getContractFactory("ArbitrageExecutor");
  const arbitrageExecutor = await ArbitrageExecutor.deploy(
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