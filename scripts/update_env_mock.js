// Script to update the .env file with mock addresses
const fs = require('fs');
const path = require('path');

// Mock addresses
const mockAddresses = {
  arbitrageExecutor: "0x0000000000000000000000000000000000000001",
  lendingPool: "0x0000000000000000000000000000000000000002",
  uniswapRouter: "0x0000000000000000000000000000000000000003",
  sushiswapRouter: "0x0000000000000000000000000000000000000004",
  curveRouter: "0x0000000000000000000000000000000000000005",
  weth: "0x0000000000000000000000000000000000000006",
  usdc: "0x0000000000000000000000000000000000000007",
  dai: "0x0000000000000000000000000000000000000008"
};

// Save the addresses to a file for easy access
fs.writeFileSync("test-addresses.json", JSON.stringify(mockAddresses, null, 2));
console.log("Mock addresses saved to test-addresses.json");

// Read the current .env file
const envPath = path.join('mev_arbitrage_bot', '.env');
let envContent = fs.readFileSync(envPath, 'utf8');

// Update the addresses in the .env file
envContent = envContent.replace(
  /ETHEREUM_RPC_URL=.*/,
  'ETHEREUM_RPC_URL=http://localhost:8545'
);
envContent = envContent.replace(
  /ETHEREUM_WS_URL=.*/,
  'ETHEREUM_WS_URL=ws://localhost:8545'
);
envContent = envContent.replace(
  /ETHEREUM_CHAIN_ID=.*/,
  'ETHEREUM_CHAIN_ID=31337'
);
envContent = envContent.replace(
  /CONTRACT_ADDRESS=.*/,
  `CONTRACT_ADDRESS=${mockAddresses.arbitrageExecutor}`
);
envContent = envContent.replace(
  /AAVE_LENDING_POOL_ADDRESS=.*/,
  `AAVE_LENDING_POOL_ADDRESS=${mockAddresses.lendingPool}`
);
envContent = envContent.replace(
  /UNISWAP_ROUTER_ADDRESS=.*/,
  `UNISWAP_ROUTER_ADDRESS=${mockAddresses.uniswapRouter}`
);
envContent = envContent.replace(
  /SUSHISWAP_ROUTER_ADDRESS=.*/,
  `SUSHISWAP_ROUTER_ADDRESS=${mockAddresses.sushiswapRouter}`
);
envContent = envContent.replace(
  /CURVE_ROUTER_ADDRESS=.*/,
  `CURVE_ROUTER_ADDRESS=${mockAddresses.curveRouter}`
);

// Write the updated .env file
fs.writeFileSync(envPath, envContent);
console.log("Updated .env with mock addresses");

// Update the config.test.toml file
const configPath = path.join('mev_arbitrage_bot', 'config.test.toml');
let configContent = fs.readFileSync(configPath, 'utf8');

// Update the addresses in the config.test.toml file
configContent = configContent.replace(
  /aave_lending_pool = "0x0000000000000000000000000000000000000000"/,
  `aave_lending_pool = "${mockAddresses.lendingPool}"`
);

// Update token addresses
configContent = configContent.replace(
  /symbol = "WETH"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "WETH"\naddress = "${mockAddresses.weth}"`
);
configContent = configContent.replace(
  /symbol = "USDC"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "USDC"\naddress = "${mockAddresses.usdc}"`
);
configContent = configContent.replace(
  /symbol = "DAI"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "DAI"\naddress = "${mockAddresses.dai}"`
);

// Update DEX addresses
configContent = configContent.replace(
  /\[dex\.uniswap\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.uniswap]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "${mockAddresses.uniswapRouter}"`
);
configContent = configContent.replace(
  /\[dex\.sushiswap\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.sushiswap]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "${mockAddresses.sushiswapRouter}"`
);
configContent = configContent.replace(
  /\[dex\.curve\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.curve]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "${mockAddresses.curveRouter}"`
);

// Write the updated config.test.toml file
fs.writeFileSync(configPath, configContent);
console.log("Updated config.test.toml with mock addresses");