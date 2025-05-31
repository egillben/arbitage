// Simple script to deploy the ArbitrageExecutor contract
const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Deploying ArbitrageExecutor contract...");
  
  // Connect to the local hardhat node
  const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");
  
  // Get the first account as the deployer
  const accounts = await provider.listAccounts();
  const deployer = provider.getSigner(accounts[0]);
  const deployerAddress = await deployer.getAddress();
  console.log("Deploying with account:", deployerAddress);
  
  // Mock addresses for dependencies
  const lendingPoolAddress = "0x0000000000000000000000000000000000000001";
  const uniswapRouterAddress = "0x0000000000000000000000000000000000000002";
  const sushiswapRouterAddress = "0x0000000000000000000000000000000000000003";
  const curveRouterAddress = "0x0000000000000000000000000000000000000004";
  
  // Load the ArbitrageExecutor contract artifact
  const artifactPath = path.join(__dirname, '../../artifacts/contracts/ArbitrageExecutor.sol/ArbitrageExecutor.json');
  const artifactRaw = fs.readFileSync(artifactPath, 'utf8');
  const artifact = JSON.parse(artifactRaw);
  
  // Deploy the ArbitrageExecutor contract
  console.log("Deploying ArbitrageExecutor...");
  const ArbitrageExecutorFactory = new ethers.ContractFactory(
    artifact.abi,
    artifact.bytecode,
    deployer
  );
  
  const arbitrageExecutor = await ArbitrageExecutorFactory.deploy(
    lendingPoolAddress,
    uniswapRouterAddress,
    sushiswapRouterAddress,
    curveRouterAddress
  );
  
  await arbitrageExecutor.deployed();
  console.log("ArbitrageExecutor deployed to:", arbitrageExecutor.address);
  
  // Update the config.test_contracts.json file
  const configPath = path.join(__dirname, '../../mev_arbitrage_bot/config.test_contracts.json');
  const configRaw = fs.readFileSync(configPath, 'utf8');
  const config = JSON.parse(configRaw);
  
  // Update the ArbitrageExecutor address
  config.arbitrage.contract.contract_address = arbitrageExecutor.address;
  
  // Update the mock addresses
  config.flash_loan.aave_lending_pool = lendingPoolAddress;
  config.dex.uniswap.router_address = uniswapRouterAddress;
  config.dex.sushiswap.router_address = sushiswapRouterAddress;
  config.dex.curve.router_address = curveRouterAddress;
  
  // Write the updated config back to the file
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log("Updated config.test_contracts.json with deployed addresses");
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });