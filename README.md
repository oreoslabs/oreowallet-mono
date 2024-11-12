## 1. Overview

This repo consists of `chain_loader`, `dservice`, `dworker`, `server` and `prover`, is the core service of `OreoWallet`.

### 1.1 crates/server
Core service stores imported viewKeys from users and serves as data provider of OreoWallet.

### 1.2 crates/prover
Standalone service to generate zk proof for user transactions, serves as prover of OreoWallet.

### 1.3 crates/dservice
Quickscan server to schedule decryption tasks among all connected dworkers.

### 1.4 crates/dworker
Decryption worker connects to dservice and handles decryption tasks from dservice.

### 1.5 crates/chain_loader
A tool to fetch blocks from rpc and save in local db for better performance during dservice getBlocks.

## 2. Guide-level explanation

![basic arch](assets/arch_v2.png)

## Docker

Build

```bash
docker build -t oreowallet .
```

Run node:

```bash
ironfish start -d ~/.ironfish-testnet --rpc.http --rpc.http.port 9092 --rpc.http.host 0.0.0.0
```

Run

```bash
DB_USER=postgres \
DB_PASSWORD=postgres \
DB_PORT=5444 \
NODE_HOST=host.docker.internal \
NODE_PORT=9092 \
SECRET_KEY=a0882c5ac5e2fa771dde52b2d5639734a4411df14f4748c6f991a96e5dd9f997 \
PUBLIC_KEY=03221b2a0ebd9d6798aadee2861a5307ced1a33d143f34c571a98ab4fa534b7d3e \
SERVER_PORT=8080 \
docker-compose up
```

Or you can use up with local .env file by copying .env.local to .env:

```bash
cp .env.local .env
docker compose up --build
```

Tips

This came in handy for me because I had a different version of postgres running. To remove all previous data: 
```bash
docker compose down -v
```
