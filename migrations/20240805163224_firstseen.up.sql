-- Add up migration script here
CREATE TABLE wallet.firstseen (
    address CHAR(64) NOT NULL,
    paid boolean DEFAULT false,
    CONSTRAINT seen_pkey PRIMARY KEY (address)
);