# Claude Development Notes

## Important Commands

### Testing
- To run tests: `anchor test`
- Do NOT use `anchor test -- --testNamePattern=...` - it doesn't work
- To run specific test file: `npx jest path/to/test.spec.ts`

### Build
- To build: `anchor build`

## Common Issues
- "byte array longer than desired length" error in BN serialization
- Transaction already processed errors - fixed by using beforeEach instead of beforeAll