A simple example of how to use libfloresta on the Web. This example uses the [floresta](https://github.com/Davidson-Souza/floresta) library to create a simple node that keeps up with the network, and allows verification of utreexo proofs.

## Dependencies for building locally

- [floresta-chain](https://github.com/Davidson-Souza/floresta) (required)
- [just](https://crates.io/crates/just) (optional)
- [wasm-pack](https://crates.io/crates/wasm-pack) (required)
- [npm](https://www.npmjs.com/) (optional)
## Getting Started
You can build the WebAssembly files locally, or download them from NPM. It shouldn't make a difference.

```bash
just build
```

or if you're using NPM

```bash
npm install
```

## Running the application
This one requires Rust to be installed. You can install it with [rustup](https://rustup.rs/).
```bash
just serve
```

or if you're using NPM. This one doesn't require Rust to be installed.

```bash
npm run serve
```

This will start a local server on port 8080. You can access the application at http://localhost:8080.

## How does it work?

The application is a simple web page that uses the floresta Rust crate to create a node that connects to the network. It also uses the a Wasm lib to create a WebAssembly module that can be used by the browser to verify utreexo proofs. To get the block data, the application uses a simple JSON RPC call to a bridge node. You can read more about it [here](https://github.com/Davidson-Souza/bridge).

All the logic is inside [main.js](main.js), it initiates and updates the simple UI, and also handles the RPC calls to the bridge node. Validation and processing of the block data is done inside the WebAssembly module, and the results are returned to the main thread.

To learn more about the FlorestaChain module, thake a look at the TypeScript definitions inside the package. To learn more about Floresta in general, take a look at the [floresta](https://github.com/Davidson-Souza/floresta) repository or my blog post [here](https://blog.dlsouza.lol/2023/07/07/libfloresta.html).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details