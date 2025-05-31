// Simple script to create an arbitrage opportunity using ethers.js
const { ethers } = require('ethers');
const fs = require('fs');

async function main() {
  console.log("Creating arbitrage opportunity...");
  
  // Connect to the local Ethereum node
  const provider = new ethers.providers.JsonRpcProvider('http://localhost:8545');
  
  // Get the first account as the signer
  const accounts = await provider.listAccounts();
  const signer = provider.getSigner(accounts[0]);
  console.log("Using account:", accounts[0]);
  
  // Load the test addresses
  let testAddresses;
  try {
    testAddresses = JSON.parse(fs.readFileSync("test-addresses.json", "utf8"));
  } catch (error) {
    console.error("Error loading test addresses:", error);
    process.exit(1);
  }
  
  // ABI for ERC20 and Router
  const erc20Abi = [
    "function approve(address spender, uint256 amount) external returns (bool)",
    "function balanceOf(address account) external view returns (uint256)"
  ];
  
  const routerAbi = [
    "function getAmountsOut(uint amountIn, address[] memory path) public view returns (uint[] memory amounts)",
    "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)"
  ];
  
  // Connect to the contracts
  const weth = new ethers.Contract(testAddresses.weth, erc20Abi, signer);
  const usdc = new ethers.Contract(testAddresses.usdc, erc20Abi, signer);
  const uniswapRouter = new ethers.Contract(testAddresses.uniswapRouter, routerAbi, signer);
  const sushiswapRouter = new ethers.Contract(testAddresses.sushiswapRouter, routerAbi, signer);
  
  // Check initial prices
  console.log("\nChecking initial prices...");
  
  // Get initial price on Uniswap (WETH -> USDC)
  const uniswapAmountOut = await uniswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const initialUniswapPrice = ethers.utils.formatUnits(uniswapAmountOut[1], 6); // Convert from USDC decimals
  console.log(`Initial Uniswap price: 1 WETH = ${initialUniswapPrice} USDC`);
  
  // Get initial price on Sushiswap (WETH -> USDC)
  const sushiswapAmountOut = await sushiswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const initialSushiswapPrice = ethers.utils.formatUnits(sushiswapAmountOut[1], 6); // Convert from USDC decimals
  console.log(`Initial Sushiswap price: 1 WETH = ${initialSushiswapPrice} USDC`);
  
  // Calculate initial price difference
  const initialPriceDiff = Math.abs(parseFloat(initialUniswapPrice) - parseFloat(initialSushiswapPrice));
  const initialPriceDiffPercent = (initialPriceDiff / parseFloat(initialUniswapPrice)) * 100;
  console.log(`Initial price difference: ${initialPriceDiffPercent.toFixed(2)}%`);
  
  // Create a significant price discrepancy by executing a large trade on Uniswap
  console.log("\nCreating price discrepancy...");
  
  // Approve WETH for the router
  const tradeAmount = ethers.utils.parseEther("20"); // 20 WETH
  await weth.approve(uniswapRouter.address, tradeAmount);
  console.log("Approved WETH for Uniswap Router");
  
  // Execute a large WETH -> USDC swap on Uniswap to decrease WETH price (increase USDC price)
  console.log("Executing large WETH -> USDC swap on Uniswap...");
  const tx = await uniswapRouter.swapExactTokensForTokens(
    tradeAmount,
    0, // Accept any amount of USDC
    [testAddresses.weth, testAddresses.usdc],
    accounts[0],
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
  const newUniswapPrice = ethers.utils.formatUnits(newUniswapAmountOut[1], 6); // Convert from USDC decimals
  console.log(`New Uniswap price: 1 WETH = ${newUniswapPrice} USDC`);
  
  // Get new price on Sushiswap (WETH -> USDC)
  const newSushiswapAmountOut = await sushiswapRouter.getAmountsOut(
    ethers.utils.parseEther("1"), // 1 WETH
    [testAddresses.weth, testAddresses.usdc]
  );
  const newSushiswapPrice = ethers.utils.formatUnits(newSushiswapAmountOut[1], 6); // Convert from USDC decimals
  console.log(`New Sushiswap price: 1 WETH = ${newSushiswapPrice} USDC`);
  
  // Calculate new price difference
  const newPriceDiff = Math.abs(parseFloat(newUniswapPrice) - parseFloat(newSushiswapPrice));
  const newPriceDiffPercent = (newPriceDiff / Math.min(parseFloat(newUniswapPrice), parseFloat(newSushiswapPrice))) * 100;
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