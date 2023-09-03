build:
    wasm-pack build --release --target web

serve: build
    python3 -m http.server 8000
