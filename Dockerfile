# Use official Anchor Docker image
FROM solanafoundation/anchor:v0.32.1

# Install pnpm
RUN npm install -g pnpm

# Create Solana keypair for testing
RUN mkdir -p /root/.config/solana && \
    solana-keygen new --no-bip39-passphrase --force -o /root/.config/solana/id.json

# Set working directory
WORKDIR /workspace

# Copy package files
COPY package.json pnpm-lock.yaml* ./

# Install dependencies with pnpm
RUN pnpm install --frozen-lockfile || pnpm install

# Copy rest of project
COPY . .

# Default command: build and test with Vitest and bankrun (no validator needed)
CMD ["sh", "-c", "anchor build && NODE_OPTIONS='--max-old-space-size=2048' pnpm exec vitest run"]
