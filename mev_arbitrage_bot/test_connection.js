// Test script to verify connection to the local Hardhat node
const { ethers } = require('ethers');
const fs = require('fs');

async function main() {
  try {
    console.log("Testing connection to local Hardhat node...");
    
    // Connect to the local Hardhat node
    const provider = new ethers.providers.JsonRpcProvider('http://localhost:8545');
    
    // Get the network information
    const network = await provider.getNetwork();
    console.log("Connected to network:", {
      name: network.name,
      chainId: network.chainId
    });
    
    // Get the block number
    const blockNumber = await provider.getBlockNumber();
    console.log("Current block number:", blockNumber);
    
    // Get the accounts
    const accounts = await provider.listAccounts();
    console.log("Available accounts:", accounts);
    
    // Get the balance of the first account
    const balance = await provider.getBalance(accounts[0]);
    console.log("Balance of first account:", ethers.utils.formatEther(balance), "ETH");
    
    console.log("Connection test successful!");
    return true;
  } catch (error) {
    console.error("Connection test failed:", error.message);
    return false;
  }
}

main()
  .then((success) => {
    process.exit(success ? 0 : 1);
  })
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });