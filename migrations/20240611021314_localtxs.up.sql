-- Add up migration script here
CREATE TABLE wallet.txs (
    hash CHAR(64) NOT NULL,
    block_hash CHAR(64) NOT NULL,
    serialized_notes text [],
    CONSTRAINT txt_pkey PRIMARY KEY (hash)
);