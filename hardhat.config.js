/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
  solidity: {
    version: "0.8.19",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200
      },
      viaIR: true
    }
  },
  networks: {
    hardhat: {
      // Local testnet configuration
      chainId: 31337,
      mining: {
        auto: true,
        interval: 5000 // Mine a block every 5 seconds
      },
      accounts: {
        mnemonic: "test test test test test test test test test test test junk",
        path: "m/44'/60'/0'/0",
        count: 10,
        accountsBalance: "10000000000000000000000" // 10000 ETH per account
      }
    },
    // Add other networks as needed
    // mainnet: {
    //   url: process.env.MAINNET_URL || "",
    //   accounts: process.env.PRIVATE_KEY !== undefined ? [process.env.PRIVATE_KEY] : []
    // }
  },
  paths: {
    sources: "./contracts",
    tests: "./test",
    cache: "./cache",
    artifacts: "./artifacts"
  },
  mocha: {
    timeout: 40000
  },
  etherscan: {
    // Your API key for Etherscan
    // apiKey: process.env.ETHERSCAN_API_KEY
  }
};