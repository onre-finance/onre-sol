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
- Transaction already processed errors - if you encounter this error, it is probably caused by Bankrun optimizing
  transactions. If you send multiple identical transactions in a row (for example in a loop), Bankrun will only
  process the first one and subsequent ones will fail with this error. To fix this, you need to differentiate
  transactions in some way. For example, you can change the transaction amount or change the fee payer.
- Try to avoid using try/catch blocks in test. When trying to validate a failing case, use
  await expect(program.methods...).rejects.toThrow("Error message")