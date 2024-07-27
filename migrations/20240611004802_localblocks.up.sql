-- Add up migration script here
CREATE TABLE wallet.blocks (
    hash CHAR(64) NOT NULL,
    sequence BIGINT NOT NULL,
    transactions JSON,
    CONSTRAINT block_pkey PRIMARY KEY (sequence)
);