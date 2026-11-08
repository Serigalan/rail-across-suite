# native — the core's Rust crates as one WebAssembly module

`packages/core/src/native/` · depends on nothing else in the core

Decision 0020 moves the core to Rust module by module. The crates (`crates/` of the repository) are built
as one WebAssembly module, `crates/core-wasm`, and this module is the face the JavaScript calls it
through: a request `{ op, input }` as JSON in, `{ ok }` or `{ error, detail }` out — the protocol the
simulation and the optimizer already use, with no bindings generator.

The module is loaded **once**, and every call after that is synchronous, as the core's calls always were:

- in the browser, a page awaits `loadNative(url)` before it renders;
- in Node — the server, the tests — `loadNativeFile()` (`./node.js`) reads `packages/core/wasm/` synchronously.

Where it is not loaded, a function that has moved stays with its JavaScript (`nativeReady()`), so the core
answers the same either way while the port goes on; the tests hold both to the same answers.

## Files

| File | What |
|---|---|
| `index.js` | `loadNative`, `useNativeModule`, `callNative`, `nativeReady`, `unloadNative`, `NativeError` |
| `node.js` | `loadNativeFile` — the module from its file, synchronously (Node only) |

`npm run wasm` (`scripts/wasm.mjs`) builds the module with cargo and puts it in `packages/core/wasm/`
(not in the repository); `RA_CORE_WASM` names a module file to take instead.
