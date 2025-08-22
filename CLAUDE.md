# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Claude Development Notes

## Important Commands

- To run tests: `anchor test`
- Do NOT use `anchor test -- --testNamePattern=...` - it doesn't work

### Building
```bash
# Build the Anchor program
anchor build
```

### Testing
```bash
# Run all tests (includes building program and copying to fixtures)
anchor test

# Test single file
npx jest --runInBand path/to/test.spec.ts
```

## Common Issues
- "byte array longer than desired length" error in BN serialization
- Transaction already processed errors - fixed by using beforeEach instead of beforeAll