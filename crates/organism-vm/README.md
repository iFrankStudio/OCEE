# OCEE-0 typed organism VM — P2 initial slice

A small deterministic interpreter for type-checked, bounded program graphs, deliberately separated from the world kernel. It has no socket, file, clock, FFI or AI service API.

### Implemented

- Versioned `Program -> Function -> Block -> typed instruction/terminator` representation.
- Static operand/type checking, block-parameter arity checking, control-flow cycle rejection, size limits.
- Deterministic arithmetic, byte slicing/concatenation, SHA-256 and hex encoding.
- Canonical JSON bytes with sorted object keys and domain-separated content hash:
  `SHA-256("OCEE-PROGRAM-V1\\0" || canonical_json)`.
- Pure isolated evaluation, bounded gas usage and metered runtime faults.
- `RepeatBounded` CFG header (up to 256 iterations), with loop counters persisted in continuation and unbounded cycles forbidden.
- `Continuation` serialized across tick boundaries; `YIELD` emits a typed world intent with feedback injected upon resumption.
- A typed, validated `mutate_literal` example for candidate version creation; no automatic strategy selection.

### Deliberate limitations

- Only `Bool`, `U32`, `Bytes` and **one function per program** currently supported; cross-function `CALL`, `Unit` and `Result<T>` are pending. Generic CFG cycles are rejected unless controlled by a statically bounded loop header.
- Graph edges must be acyclic. This is a P2 slice, not a full implementation of all instruction forms in engineering contract 04.
- The standalone `run_isolated` helper permits pure computation; world-kernel's `EXEC` bridge additionally permits metered, authorized local snapshot `OBSERVE/READ`, while rejecting SEND/WRITE and other side effects.
- The VM does not invent or choose useful programs. Any exploration strategy must be explicitly implemented and credited to the organism initial seed or experimentally developed behavior.
- Version storage, switching and world effects are exclusively handled by the world kernel.

### Run

```sh
cargo test --workspace
```

Do not present the engineering fixture as emergent cognition, life, or open-ended evolution.
