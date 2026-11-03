# Install pinned stable tools; a published Anchor 1.2.2 image is not available.
FROM solanafoundation/anchor:v1.0.2 AS toolchain
RUN rustup toolchain install 1.99.1 --profile minimal && rustup default 1.99.1 && \
    cargo install --git https://github.com/otter-sec/anchor --tag v1.2.2 anchor-cli --locked --force && \
    sh -c "$(curl -sSfL https://release.anza.xyz/v4.1.2/install)"

# Stage 1: Build Solana program with Anchor
FROM toolchain AS builder

WORKDIR /workspace
COPY . .
RUN anchor build --arch v3 && \
    # Clean up build artifacts to save disk space
    rm -rf target/debug target/deploy/*.txt target/deploy/*.dump target/release/deps target/release/build && \
    cargo clean --release --target-dir target/sbpfv3-solana-solana || true

# Stage 2: Run Rust LiteSVM tests with the same Anchor toolchain.
FROM toolchain

WORKDIR /workspace

COPY . .
COPY --from=builder /workspace/target/deploy ./target/deploy
COPY --from=builder /workspace/target/idl ./target/idl

# Default command: run Rust tests and copy artifacts to output.
CMD ["bash", "-lc", "cargo test --manifest-path programs/onreapp/Cargo.toml --tests && mkdir -p /workspace/output && cp -r target/deploy target/idl /workspace/output/"]
