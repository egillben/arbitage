# MEV Arbitrage Bot

A Rust-based MEV arbitrage bot that uses flash loans to execute profitable arbitrage opportunities on Ethereum. The bot integrates with MEV-Share for protection against front-running and uses Alchemy for blockchain connectivity. It works in conjunction with a Solidity smart contract for executing arbitrage transactions.

## Architecture

The system consists of these core components:

- **Opportunity Scanner**: Monitors DEX prices and identifies arbitrage opportunities
- **Arbitrage Strategy Engine**: Evaluates opportunities and determines optimal trade paths
- **Flash Loan Manager**: Interfaces with Aave flash loan contracts
- **Transaction Builder**: Constructs transaction payloads
- **Gas Price Optimizer**: Calculates optimal gas prices
- **Transaction Executor**: Submits transactions to the Ethereum network
- **Blockchain Event Listener**: Processes blockchain events
- **Price Oracle**: Maintains price data
- **Contract Manager**: Handles interaction with the ArbitrageExecutor smart contract
- **MEV-Share Client**: Interfaces with the MEV-Share network for private transactions

External integrations include:
- Alchemy API for enhanced blockchain connectivity
- MEV-Share network via mev-share-rs for protection against front-running
- DEX interfaces (Uniswap, Sushiswap, Curve)
- Aave flash loan interface
- ArbitrageExecutor smart contract for on-chain execution

## Security Features

The bot implements several security measures:
- Protection against front-running using MEV-Share
- Smart contract security best practices
- Multiple price sources to prevent oracle manipulation
- Strict validation of flash loan callbacks
- Dynamic gas price adjustment
- Transaction timeout mechanisms
- Secure key management

## Performance Optimizations

The bot is optimized for performance in these areas:
- Efficient blockchain data processing
- Optimized opportunity detection algorithms
- Gas-efficient smart contracts
- Asynchronous processing where appropriate

## Getting Started

### Prerequisites

- Rust (latest stable version)
- An Ethereum node provider (e.g., Alchemy)
- Alchemy API key
- MEV-Share API key
- Ethereum wallet with private key and funds for gas

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/mev_arbitrage_bot.git
cd mev_arbitrage_bot
```

2. Build the project:
```bash
cargo build --release
```

### Configuration

Create a `.env` file in the project root with the following variables:

```
# Ethereum Configuration
ETHEREUM_RPC_URL=https://eth-mainnet.alchemyapi.io/v2/your-api-key
ETHEREUM_WS_URL=wss://eth-mainnet.ws.alchemyapi.io/v2/your-api-key
ETHEREUM_PRIVATE_KEY=your-private-key
ETHEREUM_CHAIN_ID=1

# Alchemy API Configuration
ALCHEMY_API_KEY=your-alchemy-api-key

# MEV-Share Configuration
MEV_SHARE_API_KEY=your-mev-share-api-key
MEV_SHARE_API_URL=https://mev-share.flashbots.net

# Smart Contract Configuration
CONTRACT_ADDRESS=0x0000000000000000000000000000000000000000
DEPLOY_CONTRACT_IF_MISSING=true
```

Alternatively, you can create a `config.toml` file with more detailed configuration options. See `config.rs` for available options.

### Smart Contract Deployment

The bot can either use an existing ArbitrageExecutor contract or deploy a new one. To use an existing contract, set the `CONTRACT_ADDRESS` environment variable. To deploy a new contract, set `DEPLOY_CONTRACT_IF_MISSING=true`.

To manually deploy the contract:

```bash
# Using Hardhat
cd contracts
npm install
npx hardhat compile
npx hardhat deploy --network mainnet
```

The contract requires the following parameters during deployment:
- Aave Lending Pool address
- Uniswap Router address
- Sushiswap Router address
- Curve Router address

### Running the Bot

```bash
cargo run --release
```

## Project Structure

```
arbitage/
├── mev_arbitrage_bot/          # Rust implementation
│   ├── src/
│   │   ├── main.rs             # Entry point
│   │   ├── config.rs           # Configuration
│   │   ├── contract/           # Smart contract integration
│   │   ├── scanner/            # Opportunity scanner
│   │   ├── strategy/           # Arbitrage strategy engine
│   │   ├── flash_loan/         # Flash loan manager
│   │   ├── transaction/        # Transaction builder and executor
│   │   ├── gas/                # Gas price optimizer
│   │   ├── blockchain/         # Blockchain interaction and event listener
│   │   ├── price/              # Price oracle
│   │   ├── dex/                # DEX interfaces
│   │   ├── mev_share/          # MEV-Share integration
│   │   └── utils/              # Utility functions
│   └── Cargo.toml              # Project manifest
│
└── contracts/                  # Solidity smart contracts
    ├── ArbitrageExecutor.sol   # Main arbitrage contract
    ├── interfaces/             # Contract interfaces
    ├── libraries/              # Contract libraries
    ├── scripts/                # Deployment scripts
    └── test/                   # Contract tests
```

## Alchemy API Integration

The bot uses Alchemy API for enhanced blockchain connectivity. It leverages the following Alchemy features:

- WebSocket subscriptions for real-time updates
- Enhanced gas price estimation
- Token balance queries
- Transaction simulation

To configure Alchemy API:

1. Create an account at [Alchemy](https://www.alchemy.com/)
2. Create a new app and get your API key
3. Set the `ALCHEMY_API_KEY` environment variable

## MEV-Share Integration

The bot uses MEV-Share for protection against front-running. It leverages the following MEV-Share features:

- Private transaction submission
- Bundle creation and submission
- Transaction hints for privacy

To configure MEV-Share:

1. Get an API key from [Flashbots](https://www.flashbots.net/)
2. Set the `MEV_SHARE_API_KEY` environment variable

## Smart Contract Integration

The bot interacts with the ArbitrageExecutor smart contract to execute arbitrage opportunities. The contract:

1. Receives flash loans from Aave
2. Executes trades across multiple DEXes (Uniswap, Sushiswap, Curve)
3. Repays the flash loan with a profit
4. Includes safety features like emergency stop and authorized callers

The Rust bot:
1. Identifies arbitrage opportunities
2. Calculates optimal trade paths and amounts
3. Calls the smart contract with the appropriate parameters
4. Monitors transaction status and results

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Disclaimer

This software is for educational purposes only. Use at your own risk. The authors are not responsible for any financial losses incurred from using this software.

Trading cryptocurrencies involves significant risk and can result in the loss of your invested capital. You should not invest more than you can afford to lose and should ensure that you fully understand the risks involved.