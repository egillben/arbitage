// Script to create a significant arbitrage opportunity between Uniswap and Sushiswap
const { ethers } = require("hardhat");
const fs = require("fs");

async function main() {
  console.log("Creating arbitrage opportunity...");
  
  // Get signers
  const [deployer] = await ethers.getSigners();
  console.log("Using account:", deployer.address);
  
  // Load the test addresses
  let testAddresses;
  try {
    testAddresses = JSON.parse(fs.readFileSync("test-addresses.json", "utf8"));
  } catch (error) {
    console.error("Error loading test addresses:", error);
    process.exit(1);
  }
  
  // Connect to the contracts
  const weth = await ethers.getContractAt("TestERC20", testAddresses.weth);
  const usdc = await ethers.getContractAt("TestERC20", testAddresses.usdc);
  const uniswapRouter = await ethers.getContractAt("TestUniswapV2Router", testAddresses.uniswapRouter);
  const sushiswapRouter = await ethers.getContractAt("TestUniswapV2Router", testAddresses.sushiswapRouter);
  
  // Check initial prices
  console.log("\nChecking initial prices...");
  
  // Get initial price on Uniswap (WETH -> USDC)
  const uniswapAmountOut = await uniswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const initialUniswapPrice = uniswapAmountOut[1] / 1e6; // Convert from USDC decimals
  console.log(`Initial Uniswap price: 1 WETH = ${initialUniswapPrice} USDC`);
  
  // Get initial price on Sushiswap (WETH -> USDC)
  const sushiswapAmountOut = await sushiswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const initialSushiswapPrice = sushiswapAmountOut[1] / 1e6; // Convert from USDC decimals
  console.log(`Initial Sushiswap price: 1 WETH = ${initialSushiswapPrice} USDC`);
  
  // Calculate initial price difference
  const initialPriceDiff = Math.abs(initialUniswapPrice - initialSushiswapPrice);
  const initialPriceDiffPercent = (initialPriceDiff / initialUniswapPrice) * 100;
  console.log(`Initial price difference: ${initialPriceDiffPercent.toFixed(2)}%`);
  
  // Create a significant price discrepancy by executing a large trade on Uniswap
  console.log("\nCreating price discrepancy...");
  
  // Approve WETH for the router
  const tradeAmount = ethers.utils.parseEther("20"); // 20 WETH
  await weth.approve(uniswapRouter.address, tradeAmount);
  
  // Execute a large WETH -> USDC swap on Uniswap to decrease WETH price (increase USDC price)
  console.log("Executing large WETH -> USDC swap on Uniswap...");
  const tx = await uniswapRouter.swapExactTokensForTokens(
    tradeAmount,
    0, // Accept any amount of USDC
    [testAddresses.weth, testAddresses.usdc],
    deployer.address,
    Math.floor(Date.now() / 1000) + 3600 // 1 hour deadline
  );
  
  await tx.wait();
  console.log("Swap executed successfully");
  
  // Check new prices
  console.log("\nChecking new prices...");
  
  // Get new price on Uniswap (WETH -> USDC)
  const newUniswapAmountOut = await uniswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const newUniswapPrice = newUniswapAmountOut[1] / 1e6; // Convert from USDC decimals
  console.log(`New Uniswap price: 1 WETH = ${newUniswapPrice} USDC`);
  
  // Get new price on Sushiswap (WETH -> USDC)
  const newSushiswapAmountOut = await sushiswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const newSushiswapPrice = newSushiswapAmountOut[1] / 1e6; // Convert from USDC decimals
  console.log(`New Sushiswap price: 1 WETH = ${newSushiswapPrice} USDC`);
  
  // Calculate new price difference
  const newPriceDiff = Math.abs(newUniswapPrice - newSushiswapPrice);
  const newPriceDiffPercent = (newPriceDiff / Math.min(newUniswapPrice, newSushiswapPrice)) * 100;
  console.log(`New price difference: ${newPriceDiffPercent.toFixed(2)}%`);
  
  if (newPriceDiffPercent >= 5) {
    console.log("\nSuccessfully created an arbitrage opportunity with >5% price difference!");
    console.log("The MEV bot should detect and attempt to exploit this opportunity.");
  } else {
    console.log("\nPrice difference is less than 5%. You may need to execute a larger trade.");
  }
}

// Execute the script
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });