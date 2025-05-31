// Script to create a configuration file with hardcoded addresses
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("Creating configuration file with hardcoded addresses...");
  
  // Hardcoded addresses for testing
  const addresses = {
    weth: "0x5FbDB2315678afecb367f032d93F642f64180aa3",
    usdc: "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
    dai: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0",
    uniswapFactory: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9",
    uniswapRouter: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9",
    sushiswapFactory: "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707",
    sushiswapRouter: "0x0165878A594ca255338adfa4d48449f69242Eb8F",
    curveFactory: "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853",
    curveRouter: "0x2279B7A0a67DB372996a5FaB50D91eAA73d2eBe6",
    lendingPool: "0x8A791620dd6260079BF849Dc5567aDC3F2FdC318",
    arbitrageExecutor: "0x610178dA211FEF7D417bC0e6FeD39F05609AD788"
  };
  
  // Create the config object in the format expected by the bot
  const configJson = {
    "ethereum": {
      "rpc_url": "http://localhost:8545",
      "chain_id": 31337,
      "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266", // First hardhat account
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
      "aave_lending_pool": addresses.lendingPool,
      "max_borrow_amount": 100.0,
      "tokens": [
        {
          "symbol": "WETH",
          "address": addresses.weth,
          "decimals": 18
        },
        {
          "symbol": "USDC",
          "address": addresses.usdc,
          "decimals": 6
        },
        {
          "symbol": "DAI",
          "address": addresses.dai,
          "decimals": 18
        }
      ]
    },
    "dex": {
      "uniswap": {
        "enabled": true,
        "factory_address": addresses.uniswapFactory,
        "router_address": addresses.uniswapRouter,
        "pools": [
          {
            "name": "WETH-USDC",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
            "token0": addresses.weth,
            "token1": addresses.usdc
          },
          {
            "name": "WETH-DAI",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D9",
            "token0": addresses.weth,
            "token1": addresses.dai
          },
          {
            "name": "USDC-DAI",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6Da",
            "token0": addresses.usdc,
            "token1": addresses.dai
          }
        ]
      },
      "sushiswap": {
        "enabled": true,
        "factory_address": addresses.sushiswapFactory,
        "router_address": addresses.sushiswapRouter,
        "pools": [
          {
            "name": "WETH-USDC",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6Db",
            "token0": addresses.weth,
            "token1": addresses.usdc
          },
          {
            "name": "WETH-DAI",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6Dc",
            "token0": addresses.weth,
            "token1": addresses.dai
          },
          {
            "name": "USDC-DAI",
            "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6Dd",
            "token0": addresses.usdc,
            "token1": addresses.dai
          }
        ]
      },
      "curve": {
        "enabled": true,
        "factory_address": addresses.curveFactory,
        "router_address": addresses.curveRouter,
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
        "contract_address": addresses.arbitrageExecutor,
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
  fs.writeFileSync("test-addresses.json", JSON.stringify(addresses, null, 2));
  console.log("Addresses also saved to test-addresses.json");
}

// Execute the script
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });