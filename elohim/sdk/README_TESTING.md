# Holochain SDK - Testing Guide

## Quick Start

```bash
# Install dependencies
npm install

# Run all tests
npm test

# Run tests in watch mode
npm test -- --watch

# Run specific test file
npm test -- src/connection.spec.ts

# Run with coverage (when compatible vitest installed)
npm test -- --coverage
```

## Test Structure

Tests are located alongside their source files with `.spec.ts` extension:

```
src/
├── connection.ts
├── connection.spec.ts          # Tests for connection.ts
├── client/
│   ├── batch-executor.ts
│   ├── batch-executor.spec.ts  # Tests for batch-executor.ts
│   └── zome-client.ts
└── services/
    ├── content.service.ts
    ├── content.service.spec.ts # Tests for content.service.ts
    └── ...
```

## Writing Tests

### Test Template

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MyClass } from './my-class.js';
import { Dependency } from './dependency.js';

// Mock external dependencies
vi.mock('./dependency.js', () => ({
  Dependency: vi.fn(),
}));

describe('MyClass', () => {
  let instance: MyClass;
  let mockDep: any;

  beforeEach(() => {
    vi.clearAllMocks();

    mockDep = {
      someMethod: vi.fn(),
    };

    (Dependency as any).mockImplementation(() => mockDep);
    instance = new MyClass(mockDep);
  });

  describe('methodName', () => {
    it('should do something when condition is met', async () => {
      // Arrange
      mockDep.someMethod.mockResolvedValue('result');

      // Act
      const result = await instance.methodName();

      // Assert
      expect(result).toBe('result');
      expect(mockDep.someMethod).toHaveBeenCalledWith(/* expected args */);
    });

    it('should handle errors gracefully', async () => {
      // Arrange
      mockDep.someMethod.mockRejectedValue(new Error('Test error'));

      // Act & Assert
      await expect(instance.methodName()).rejects.toThrow('Test error');
    });
  });
});
```

### Best Practices

1. **Test Isolation**: Each test should be independent
2. **Meaningful Names**: Use descriptive test names that explain the scenario
3. **Arrange-Act-Assert**: Structure tests clearly
4. **Mock External Dependencies**: Don't test external libraries
5. **Test Behavior, Not Implementation**: Focus on what, not how

## Coverage Goals

- **Lines**: 50%+
- **Functions**: 50%+
- **Branches**: 50%+
- **Statements**: 50%+

## Common Patterns

### Mocking Holochain Client

```typescript
import { AdminWebsocket, AppWebsocket } from '@holochain/client';

vi.mock('@holochain/client', () => ({
  AdminWebsocket: {
    connect: vi.fn(),
  },
  AppWebsocket: {
    connect: vi.fn(),
  },
}));

beforeEach(() => {
  mockAdminWs = {
    listApps: vi.fn(),
    client: { close: vi.fn() },
  };

  (AdminWebsocket.connect as any).mockResolvedValue(mockAdminWs);
});
```

### Testing Async Operations

```typescript
it('should handle async operation', async () => {
  mockClient.callZome.mockResolvedValue({ result: 'success' });

  const result = await service.doSomething();

  expect(result).toEqual({ result: 'success' });
});
```

### Testing Error Handling

```typescript
it('should propagate errors', async () => {
  mockClient.callZome.mockRejectedValue(new Error('Network error'));

  await expect(service.doSomething()).rejects.toThrow('Network error');
});
```

## Debugging Tests

### Run Single Test

```bash
npm test -- src/connection.spec.ts
```

### Run Tests Matching Pattern

```bash
npm test -- --grep "connection"
```

### Verbose Output

```bash
npm test -- --reporter=verbose
```

### Debug in VS Code

Add to `.vscode/launch.json`:

```json
{
  "type": "node",
  "request": "launch",
  "name": "Debug Tests",
  "runtimeExecutable": "npm",
  "runtimeArgs": ["test", "--", "--run"],
  "console": "integratedTerminal"
}
```

## Troubleshooting

### Tests Timeout

Increase timeout in test:

```typescript
it('slow test', async () => {
  // test code
}, 10000); // 10 second timeout
```

### Mock Not Working

Ensure mock is defined before import:

```typescript
// ✅ Correct order
vi.mock('./dependency.js');
import { MyClass } from './my-class.js';

// ❌ Wrong order
import { MyClass } from './my-class.js';
vi.mock('./dependency.js');
```

### Coverage Not Generating

Check vitest version compatibility:

```bash
npm list vitest
npm list @vitest/coverage-v8
```

Both should be compatible versions (same major.minor).

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run tests
  run: npm test -- --run --reporter=dot

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    files: ./coverage/lcov.info
```

### Pre-commit Hook

```bash
#!/bin/sh
npm test -- --run --reporter=dot
```

## Adding New Tests

1. Create `*.spec.ts` file next to source file
2. Follow existing patterns from similar tests
3. Run tests locally to verify
4. Aim for at least 80% coverage of new code
5. Include happy path, error cases, and edge cases

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [Testing Best Practices](https://github.com/goldbergyoni/javascript-testing-best-practices)
- [Holochain Client API](https://github.com/holochain/holochain-client-js)
