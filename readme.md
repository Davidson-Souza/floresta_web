A simple example of how to use libfloresta on the Web. This example uses the [floresta](https://github.com/Davidson-Souza/floresta) library to create a simple node that keeps up with the network, and allows verification of utreexo proofs.

## Dependencies

- [floresta-chain](https://github.com/Davidson-Souza/floresta)
- [just](https://crates.io/crates/just)
- [wasm-pack](https://crates.io/crates/wasm-pack)
- [wasm-bindgen](https://crates.io/crates/wasm-bindgen)

## Build

```bash
just build
```

## Run

```bash
just serve
```

This will start a local server on port 8080. You can access the application at http://localhost:8080.