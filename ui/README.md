# Agora Parachain UI

A modern React-based web interface for interacting with the Agora verifiable computation marketplace parachain.

## Features

- **Chain Connection**: Connect to Agora parachain via WebSocket
- **Account Management**: Connect using Polkadot.js extension or manual address entry
- **Job Management**: Submit, view, and manage computation jobs
- **Worker Dashboard**: Register as a worker, commit/reveal results
- **Real-time Updates**: Subscribe to chain events for live updates
- **Dashboard**: View statistics and overview of the marketplace

## Setup

### Prerequisites

- Node.js 18+ and npm
- Polkadot.js Extension (for wallet connection)
- Running Agora parachain node (default: `ws://localhost:9990`)

### Installation

**Note**: If you encounter npm cache permission errors, you have two options:

**Option 1: Fix npm permissions** (Recommended)
```bash
sudo chown -R $(whoami) ~/.npm
cd ui
npm install
```

**Option 2: Use --legacy-peer-deps**
```bash
cd ui
npm install --legacy-peer-deps
```

**Option 3: Use yarn** (Alternative)
```bash
cd ui
yarn install
```

### Development

```bash
npm run dev
```

The UI will be available at `http://localhost:3000`

### Build

```bash
npm run build
```

## Configuration

Default WebSocket endpoint: `ws://localhost:9990` (Para 1000)

You can change the endpoint in `src/utils/constants.ts` or modify it at runtime via the settings page.

## Usage

1. **Connect Wallet**: Click "Connect Wallet" and select an account from Polkadot.js extension
2. **Submit Jobs**: Navigate to Jobs > Submit to create a new computation job
3. **Register as Worker**: Go to Workers to register and start participating
4. **Commit/Reveal**: Use the Commit/Reveal interface to participate in job consensus
5. **View Jobs**: Browse all jobs and their status on the Jobs page

## Project Structure

```
ui/
├── src/
│   ├── components/      # React components
│   │   ├── common/     # Common UI components
│   │   ├── jobs/       # Job-related components
│   │   ├── workers/    # Worker-related components
│   │   └── layout/     # Layout components
│   ├── context/        # React context providers
│   ├── hooks/          # Custom React hooks
│   ├── types/          # TypeScript type definitions
│   └── utils/          # Utility functions
├── package.json
└── vite.config.ts
```

## Technologies

- React 18
- TypeScript
- Vite
- Tailwind CSS v4 (@tailwindcss/vite)
- Polkadot.js API
- React Router

## Troubleshooting

### npm cache permission errors
If you see `EACCES` errors during installation, run:
```bash
sudo chown -R $(whoami) ~/.npm
```

### Dependency conflicts
If you encounter peer dependency warnings, use:
```bash
npm install --legacy-peer-deps
```

### Connection issues
Ensure your parachain node is running and accessible at the configured WebSocket endpoint.
