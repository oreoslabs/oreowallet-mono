CREATE SCHEMA wallet;

CREATE TABLE wallet.account (
    name VARCHAR(64) NOT NULL,
    address CHAR(64) NOT NULL,
    create_head BIGINT,
    create_hash CHAR(64),
    hash CHAR(64) NOT NULL,
    head BIGINT NOT NULL,
    in_vk CHAR(64) NOT NULL,
    out_vk CHAR(64) NOT NULL,
    vk CHAR(128) NOT NULL,
    CONSTRAINT account_pkey PRIMARY KEY (address)
);