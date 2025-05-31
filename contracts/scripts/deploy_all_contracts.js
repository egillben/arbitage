// Script to deploy all necessary contracts for the test environment
const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Deploying all contracts for the test environment...");
  
  // Connect to the local hardhat node
  const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");
  
  // Get the first account as the deployer
  const accounts = await provider.listAccounts();
  const deployer = provider.getSigner(accounts[0]);
  const deployerAddress = await deployer.getAddress();
  console.log("Deploying with account:", deployerAddress);
  
  // Load contract artifacts
  const TestERC20 = require("../../artifacts/contracts/test/TestERC20.sol/TestERC20.json");
  const TestUniswapV2Factory = require("../../artifacts/contracts/test/TestUniswapV2Factory.sol/TestUniswapV2Factory.json");
  const TestUniswapV2Router = require("../../artifacts/contracts/test/TestUniswapV2Router.sol/TestUniswapV2Router.json");
  const TestLendingPool = require("../../artifacts/contracts/test/TestLendingPool.sol/TestLendingPool.json");
  const ArbitrageExecutor = require("../../artifacts/contracts/ArbitrageExecutor.sol/ArbitrageExecutor.json");
  
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
  
  // Deploy Curve Factory
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
  
  // Create pools
  console.log("\nCreating pools...");
  
  // Create WETH-USDC pool on Uniswap
  const uniswapFactoryContract = new ethers.Contract(uniswapFactory.address, TestUniswapV2Factory.abi, deployer);
  await uniswapFactoryContract.createPair(weth.address, usdc.address);
  const uniswapWethUsdcPair = await uniswapFactoryContract.getPair(weth.address, usdc.address);
  console.log("Uniswap WETH-USDC pair:", uniswapWethUsdcPair);
  
  // Create WETH-DAI pool on Uniswap
  await uniswapFactoryContract.createPair(weth.address, dai.address);
  const uniswapWethDaiPair = await uniswapFactoryContract.getPair(weth.address, dai.address);
  console.log("Uniswap WETH-DAI pair:", uniswapWethDaiPair);
  
  // Create USDC-DAI pool on Uniswap
  await uniswapFactoryContract.createPair(usdc.address, dai.address);
  const uniswapUsdcDaiPair = await uniswapFactoryContract.getPair(usdc.address, dai.address);
  console.log("Uniswap USDC-DAI pair:", uniswapUsdcDaiPair);
  
  // Create WETH-USDC pool on Sushiswap
  const sushiswapFactoryContract = new ethers.Contract(sushiswapFactory.address, TestUniswapV2Factory.abi, deployer);
  await sushiswapFactoryContract.createPair(weth.address, usdc.address);
  const sushiswapWethUsdcPair = await sushiswapFactoryContract.getPair(weth.address, usdc.address);
  console.log("Sushiswap WETH-USDC pair:", sushiswapWethUsdcPair);
  
  // Create WETH-DAI pool on Sushiswap
  await sushiswapFactoryContract.createPair(weth.address, dai.address);
  const sushiswapWethDaiPair = await sushiswapFactoryContract.getPair(weth.address, dai.address);
  console.log("Sushiswap WETH-DAI pair:", sushiswapWethDaiPair);
  
  // Create USDC-DAI pool on Sushiswap
  await sushiswapFactoryContract.createPair(usdc.address, dai.address);
  const sushiswapUsdcDaiPair = await sushiswapFactoryContract.getPair(usdc.address, dai.address);
  console.log("Sushiswap USDC-DAI pair:", sushiswapUsdcDaiPair);
  
  // Update the config.test_contracts.json file
  const configPath = path.join(__dirname, '../../mev_arbitrage_bot/config.test_contracts.json');
  const configRaw = fs.readFileSync(configPath, 'utf8');
  const config = JSON.parse(configRaw);
  
  // Update token addresses
  config.flash_loan.tokens[0].address = weth.address;
  config.flash_loan.tokens[1].address = usdc.address;
  config.flash_loan.tokens[2].address = dai.address;
  
  // Update DEX addresses
  config.flash_loan.aave_lending_pool = lendingPool.address;
  config.dex.uniswap.factory_address = uniswapFactory.address;
  config.dex.uniswap.router_address = uniswapRouter.address;
  config.dex.sushiswap.factory_address = sushiswapFactory.address;
  config.dex.sushiswap.router_address = sushiswapRouter.address;
  config.dex.curve.factory_address = curveFactory.address;
  config.dex.curve.router_address = curveRouter.address;
  
  // Update pool addresses
  config.dex.uniswap.pools[0].address = uniswapWethUsdcPair;
  config.dex.uniswap.pools[0].token0 = weth.address;
  config.dex.uniswap.pools[0].token1 = usdc.address;
  
  config.dex.uniswap.pools[1].address = uniswapWethDaiPair;
  config.dex.uniswap.pools[1].token0 = weth.address;
  config.dex.uniswap.pools[1].token1 = dai.address;
  
  config.dex.uniswap.pools[2].address = uniswapUsdcDaiPair;
  config.dex.uniswap.pools[2].token0 = usdc.address;
  config.dex.uniswap.pools[2].token1 = dai.address;
  
  config.dex.sushiswap.pools[0].address = sushiswapWethUsdcPair;
  config.dex.sushiswap.pools[0].token0 = weth.address;
  config.dex.sushiswap.pools[0].token1 = usdc.address;
  
  config.dex.sushiswap.pools[1].address = sushiswapWethDaiPair;
  config.dex.sushiswap.pools[1].token0 = weth.address;
  config.dex.sushiswap.pools[1].token1 = dai.address;
  
  config.dex.sushiswap.pools[2].address = sushiswapUsdcDaiPair;
  config.dex.sushiswap.pools[2].token0 = usdc.address;
  config.dex.sushiswap.pools[2].token1 = dai.address;
  
  // Update ArbitrageExecutor address
  config.arbitrage.contract.contract_address = arbitrageExecutor.address;
  
  // Write the updated config back to the file
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log("\nUpdated config.test_contracts.json with deployed addresses");
  
  // Also save a copy to test-addresses.json for reference
  const addresses = {
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
    uniswapWethUsdcPair: uniswapWethUsdcPair,
    uniswapWethDaiPair: uniswapWethDaiPair,
    uniswapUsdcDaiPair: uniswapUsdcDaiPair,
    sushiswapWethUsdcPair: sushiswapWethUsdcPair,
    sushiswapWethDaiPair: sushiswapWethDaiPair,
    sushiswapUsdcDaiPair: sushiswapUsdcDaiPair
  };
  fs.writeFileSync("test-addresses.json", JSON.stringify(addresses, null, 2));
  console.log("Addresses also saved to test-addresses.json");
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });