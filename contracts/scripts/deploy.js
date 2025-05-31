// Deployment script for ArbitrageExecutor contract
const hre = require("hardhat");

async function main() {
  console.log("Deploying ArbitrageExecutor contract...");

  // Get the contract factory
  const ArbitrageExecutor = await hre.ethers.getContractFactory("ArbitrageExecutor");

  // Define the constructor parameters
  // These should be replaced with actual addresses for the target network
  const lendingPoolAddress = "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9"; // Aave V2 Lending Pool on Mainnet
  const uniswapRouterAddress = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router on Mainnet
  const sushiswapRouterAddress = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"; // Sushiswap Router on Mainnet
  const curveRouterAddress = "0x8e764bE4288B842791989DB5b8ec067279829809"; // Curve Router on Mainnet

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

  // Verify the contract on Etherscan (if API key is provided)
  if (process.env.ETHERSCAN_API_KEY) {
    console.log("Waiting for block confirmations...");
    // Wait for 6 block confirmations
    await arbitrageExecutor.deployTransaction.wait(6);
    
    console.log("Verifying contract on Etherscan...");
    await hre.run("verify:verify", {
      address: arbitrageExecutor.address,
      constructorArguments: [
        lendingPoolAddress,
        uniswapRouterAddress,
        sushiswapRouterAddress,
        curveRouterAddress
      ],
    });
    console.log("Contract verified on Etherscan");
  }
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });