# Stage 1: Build Solana program with Anchor
FROM solanafoundation/anchor:v0.32.1 AS builder

WORKDIR /workspace
COPY . .
RUN anchor build && \
    # Clean up build artifacts to save disk space
    rm -rf target/deploy/*.txt target/deploy/*.dump target/release/deps target/release/build && \
    cargo clean --release --target-dir target/sbpf-solana-solana || true

# Stage 2: Run tests with newer Ubuntu (has glibc 2.38+)
FROM ubuntu:24.04

# Install Node.js, Rust, and required tools
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# Install pnpm
RUN npm install -g pnpm

WORKDIR /workspace

# Copy built program from builder stage
COPY --from=builder /workspace/target ./target
COPY --from=builder /workspace/Anchor.toml ./Anchor.toml

# Copy package files and install dependencies
COPY package.json pnpm-lock.yaml* ./
RUN pnpm install --frozen-lockfile || pnpm install && \
    # Clean pnpm cache to save disk space
    pnpm store prune

# Copy rest of project
COPY . .

# Default command: run tests and copy artifacts to output
CMD ["sh", "-c", "NODE_OPTIONS='--max-old-space-size=1024' pnpm exec vitest run --pool=threads --poolOptions.threads.maxThreads=1 --poolOptions.threads.minThreads=1 && cp -r target/deploy target/idl /workspace/output/ 2>/dev/null || true"]
