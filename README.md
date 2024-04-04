## 1. Overview

This repo consists of `server`, `prover` and `migrate`, is the core backend service of `OreoWallet`.

### 1.1 Server
Core service stores imported viewKeys from users and serves as data provider of OreoWallet.

### 1.2 Prover
Standalone service to generate zk proof for user transactions, serves as prover of OreoWallet.

### 1.3 Migrate
This feature is useful only if you are running your own data provider since the 1st version of `ironfish-server`. Then you need to migrate redis data to one of new redis struct or postgres db.

## 2. Guide-level explanation

![basic arch](assets/arch_v2.png)

## 3. Run data provider (Advanced for developer only)

### 3.1 Install

- [Install `postgresql db`](https://www.postgresql.org/download/).
- [Install rust](https://www.rust-lang.org/tools/install).
- Install sqlx-cli with `cargo install sqlx-cli`.
  
### 3.2 Init

- If you ran `server` before, you need to migrate data to new struct with `src/bin/migrate`. 
- Init postgres db with `sqlx database create` then create table with `sqlx migrate run`, check `migrations` directory for details.

### 3.3 Run with postgres db

- Create a config file for postgres db as `fixtures/postgres-config.yml`.
- Start server with db config, node config above.

## 4. Run prover

- Build.
- Run with necessary cli opts.