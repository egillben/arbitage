// Script to update the .env file with test addresses
const fs = require('fs');
const path = require('path');

// Check if test-addresses.json exists
if (!fs.existsSync('test-addresses.json')) {
  console.error('Error: test-addresses.json not found. Please run the deploy_test_env.js script first.');
  process.exit(1);
}

// Read the test addresses
const testAddresses = JSON.parse(fs.readFileSync('test-addresses.json', 'utf8'));

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
  `CONTRACT_ADDRESS=${testAddresses.arbitrageExecutor}`
);
envContent = envContent.replace(
  /AAVE_LENDING_POOL_ADDRESS=.*/,
  `AAVE_LENDING_POOL_ADDRESS=${testAddresses.lendingPool}`
);
envContent = envContent.replace(
  /UNISWAP_ROUTER_ADDRESS=.*/,
  `UNISWAP_ROUTER_ADDRESS=${testAddresses.uniswapRouter}`
);
envContent = envContent.replace(
  /SUSHISWAP_ROUTER_ADDRESS=.*/,
  `SUSHISWAP_ROUTER_ADDRESS=${testAddresses.sushiswapRouter}`
);
envContent = envContent.replace(
  /CURVE_ROUTER_ADDRESS=.*/,
  `CURVE_ROUTER_ADDRESS=${testAddresses.curveRouter}`
);

// Write the updated .env file
fs.writeFileSync(envPath, envContent);

// Update the config.test.toml file
const configPath = path.join('mev_arbitrage_bot', 'config.test.toml');
let configContent = fs.readFileSync(configPath, 'utf8');

// Update the addresses in the config.test.toml file
configContent = configContent.replace(
  /aave_lending_pool = "0x0000000000000000000000000000000000000000"/,
  `aave_lending_pool = "${testAddresses.lendingPool}"`
);

// Update token addresses
configContent = configContent.replace(
  /symbol = "WETH"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "WETH"\naddress = "${testAddresses.weth}"`
);
configContent = configContent.replace(
  /symbol = "USDC"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "USDC"\naddress = "${testAddresses.usdc}"`
);
configContent = configContent.replace(
  /symbol = "DAI"\naddress = "0x0000000000000000000000000000000000000000"/,
  `symbol = "DAI"\naddress = "${testAddresses.dai}"`
);

// Update DEX addresses
configContent = configContent.replace(
  /\[dex\.uniswap\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.uniswap]\nenabled = true\nfactory_address = "${testAddresses.uniswapFactory || '0x0000000000000000000000000000000000000000'}"\nrouter_address = "${testAddresses.uniswapRouter}"`
);
configContent = configContent.replace(
  /\[dex\.sushiswap\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.sushiswap]\nenabled = true\nfactory_address = "${testAddresses.sushiswapFactory || '0x0000000000000000000000000000000000000000'}"\nrouter_address = "${testAddresses.sushiswapRouter}"`
);
configContent = configContent.replace(
  /\[dex\.curve\]\nenabled = true\nfactory_address = "0x0000000000000000000000000000000000000000"\nrouter_address = "0x0000000000000000000000000000000000000000"/,
  `[dex.curve]\nenabled = true\nfactory_address = "${testAddresses.curveFactory || '0x0000000000000000000000000000000000000000'}"\nrouter_address = "${testAddresses.curveRouter}"`
);

// Add test_mode flag if it doesn't exist
if (!configContent.includes('test_mode = true')) {
  configContent += '\n\n# Test mode configuration\ntest_mode = true';
}

// Write the updated config.test.toml file
fs.writeFileSync(configPath, configContent);

console.log('Updated .env and config.test.toml with test addresses');