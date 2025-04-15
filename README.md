# Mini App Web3 Wallet

A comprehensive web3 wallet Telegram Mini App integration, supporting Ethereum and Solana blockchains.

## Overview

This project implements a web3 wallet within a Telegram Mini App, allowing users to manage cryptocurrency assets across Ethereum and Solana blockchains. It provides secure wallet management, token swaps, position tracking, and automated trading features.

## Project Structure

- **telegram_app**: React-based Telegram Mini App frontend
- **solana_app**: Rust service for Solana blockchain interactions
- **backend_app**: Rust API server supporting the frontend
- **telegram_bot**: Telegram Bot handling user interactions and database management

## Key Features

- Cross-chain wallet (Ethereum and Solana)
- Secure wallet generation via Turnkey and Capsule
- Token swaps via DEXs (Jupiter, Raydium, Uniswap)
- Position monitoring with take profit/stop loss automation
- Copy trading functionality
- Real-time updates via WebSockets
- Persistent storage with Telegram Cloud Storage
- User settings and preferences management

## Technologies

- **Frontend**: React, TypeScript, Tailwind CSS, Vite
- **Backend**: Rust, Tide, Tokio
- **Storage**: PostgreSQL, Redis
- **Blockchain**: Solana SDK, Ethereum libraries
- **Infrastructure**: Docker, Nginx

## Requirements

- Docker and Docker Compose
- PostgreSQL database
- Redis server
- Telegram Bot API credentials
- Solana and Ethereum RPC endpoints
- Turnkey and Capsule SDK configuration

## Setup Instructions

1. Clone the repository
2. Configure environment variables
3. Deploy with Docker Compose:
   ```
   docker-compose up -d
   ```

## Development

### Frontend (Telegram App)
```
cd telegram_app
npm install
npm run dev
```

### Backend Components (Rust)
```
cd [component_name]
cargo build
cargo run
```

## License

[Add your license information here]