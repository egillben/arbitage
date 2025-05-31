// Script to add liquidity to the pools
const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Adding liquidity to pools...");
  
  // Connect to the local hardhat node
  const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");
  
  // Get the first account as the deployer
  const accounts = await provider.listAccounts();
  const deployer = provider.getSigner(accounts[0]);
  const deployerAddress = await deployer.getAddress();
  console.log("Using account:", deployerAddress);
  
  // Load contract artifacts
  const TestERC20 = require("../../artifacts/contracts/test/TestERC20.sol/TestERC20.json");
  const TestUniswapV2Router = require("../../artifacts/contracts/test/TestUniswapV2Router.sol/TestUniswapV2Router.json");
  
  // Load the config file to get the addresses
  const configPath = path.join(__dirname, '../../mev_arbitrage_bot/config.test_contracts.json');
  const configRaw = fs.readFileSync(configPath, 'utf8');
  const config = JSON.parse(configRaw);
  
  // Get token addresses
  const wethAddress = config.flash_loan.tokens[0].address;
  const usdcAddress = config.flash_loan.tokens[1].address;
  const daiAddress = config.flash_loan.tokens[2].address;
  
  // Get router addresses
  const uniswapRouterAddress = config.dex.uniswap.router_address;
  const sushiswapRouterAddress = config.dex.sushiswap.router_address;
  
  // Create contract instances
  const weth = new ethers.Contract(wethAddress, TestERC20.abi, deployer);
  const usdc = new ethers.Contract(usdcAddress, TestERC20.abi, deployer);
  const dai = new ethers.Contract(daiAddress, TestERC20.abi, deployer);
  
  const uniswapRouter = new ethers.Contract(uniswapRouterAddress, TestUniswapV2Router.abi, deployer);
  const sushiswapRouter = new ethers.Contract(sushiswapRouterAddress, TestUniswapV2Router.abi, deployer);
  
  // Approve tokens for the routers
  console.log("\nApproving tokens for Uniswap router...");
  await weth.approve(uniswapRouterAddress, ethers.utils.parseEther("1000"));
  await usdc.approve(uniswapRouterAddress, ethers.utils.parseUnits("1000000", 6));
  await dai.approve(uniswapRouterAddress, ethers.utils.parseEther("1000000"));
  
  console.log("Approving tokens for Sushiswap router...");
  await weth.approve(sushiswapRouterAddress, ethers.utils.parseEther("1000"));
  await usdc.approve(sushiswapRouterAddress, ethers.utils.parseUnits("1000000", 6));
  await dai.approve(sushiswapRouterAddress, ethers.utils.parseEther("1000000"));
  
  // Add liquidity to Uniswap pools
  console.log("\nAdding liquidity to Uniswap WETH-USDC pool...");
  await uniswapRouter.addLiquidity(
    wethAddress,
    usdcAddress,
    ethers.utils.parseEther("100"),
    ethers.utils.parseUnits("200000", 6), // 2000 USDC per ETH
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  console.log("Adding liquidity to Uniswap WETH-DAI pool...");
  await uniswapRouter.addLiquidity(
    wethAddress,
    daiAddress,
    ethers.utils.parseEther("100"),
    ethers.utils.parseEther("200000"), // 2000 DAI per ETH
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  console.log("Adding liquidity to Uniswap USDC-DAI pool...");
  await uniswapRouter.addLiquidity(
    usdcAddress,
    daiAddress,
    ethers.utils.parseUnits("100000", 6),
    ethers.utils.parseEther("99000"), // 0.99 DAI per USDC
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  // Add liquidity to Sushiswap pools with price discrepancies
  console.log("\nAdding liquidity to Sushiswap WETH-USDC pool...");
  await sushiswapRouter.addLiquidity(
    wethAddress,
    usdcAddress,
    ethers.utils.parseEther("100"),
    ethers.utils.parseUnits("202000", 6), // 2020 USDC per ETH (1% higher)
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  console.log("Adding liquidity to Sushiswap WETH-DAI pool...");
  await sushiswapRouter.addLiquidity(
    wethAddress,
    daiAddress,
    ethers.utils.parseEther("100"),
    ethers.utils.parseEther("198000"), // 1980 DAI per ETH (1% lower)
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  console.log("Adding liquidity to Sushiswap USDC-DAI pool...");
  await sushiswapRouter.addLiquidity(
    usdcAddress,
    daiAddress,
    ethers.utils.parseUnits("100000", 6),
    ethers.utils.parseEther("101000"), // 1.01 DAI per USDC (1% higher)
    0,
    0,
    deployerAddress,
    Math.floor(Date.now() / 1000) + 3600
  );
  
  console.log("\nLiquidity added to all pools successfully!");
}

// Execute the script
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });