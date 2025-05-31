// Simple deployment script for the ArbitrageExecutor contract
const hre = require("hardhat");

async function main() {
  console.log("Deploying ArbitrageExecutor contract...");

  // Get the contract factory
  const ArbitrageExecutor = await hre.ethers.getContractFactory("ArbitrageExecutor");

  // Define the constructor parameters (mock addresses for testing)
  const lendingPoolAddress = "0x0000000000000000000000000000000000000001";
  const uniswapRouterAddress = "0x0000000000000000000000000000000000000002";
  const sushiswapRouterAddress = "0x0000000000000000000000000000000000000003";
  const curveRouterAddress = "0x0000000000000000000000000000000000000004";

  // Deploy the contract
  const arbitrageExecutor = await ArbitrageExecutor.deploy(
    lendingPoolAddress,
    uniswapRouterAddress,
    sushiswapRouterAddress,
    curveRouterAddress
  );

  // Wait for the contract to be deployed
  await arbitrageExecutor.deployed();

  console.log("ArbitrageExecutor deployed to:", arbitrageExecutor.address);
  console.log("Transaction hash:", arbitrageExecutor.deployTransaction.hash);

  // Save the address to a file for easy access
  const fs = require("fs");
  const addresses = {
    arbitrageExecutor: arbitrageExecutor.address,
    lendingPool: lendingPoolAddress,
    uniswapRouter: uniswapRouterAddress,
    sushiswapRouter: sushiswapRouterAddress,
    curveRouter: curveRouterAddress
  };

  fs.writeFileSync("test-addresses.json", JSON.stringify(addresses, null, 2));
  console.log("Addresses saved to test-addresses.json");
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });