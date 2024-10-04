# Use the official Rust image as a base
FROM rust:latest AS builder

# Set the working directory
WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the project
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the built binaries from the builder stage
COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/target/release/chain_loader /app/chain_loader
COPY --from=builder /app/target/release/dservice /app/dservice
COPY --from=builder /app/target/release/dworker /app/dworker
COPY --from=builder /app/target/release/prover /app/prover


# Expose necessary ports
EXPOSE 10002 9093 10001 20001

# Define the entry point for the container
CMD ["./main"]