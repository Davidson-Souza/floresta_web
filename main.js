/**
 * A super simple example of how to use the FlorestaChain library in a web page. This
 * example is not meant to be used in production, but should be a good starting point
 * for anyone who wants to build a web wallet, or a block explorer using the library.
 *
 * Our main component is the FlorestaChain class, which is a wrapper around the
 * Rust library. It exposes a few methods that we can use to interact with the
 * chain, and a few properties that we can use to get information about the chain.
 * This is compiled to WebAssembly, and can be used in any web page. If you need the
 * artifacts, you can build them yourself by following the instructions in the README,
 * or use the following pre-built NPM package: https://www.npmjs.com/package/example_libfloresta
 *
 * The FlorestaChain class is initialized by a constructor, you then need to pull blocks
 * from the network and add them to the chain. This is done by calling the accept_block
 * method. This method takes a JSON string as an argument, which is the block that you
 * want to add to the chain. This string is the same as the one that you get from the
 * REST API in the utreexo bridge linked in the README.
 *
 * The FlorestaChain class also has a few properties that you can use to get information
 * about the chain. Some examples are the height, tip, network, difficulty,
 * and the target for the last block. These properties are updated when you call accept_block,
 * so you can use them to update the UI.
 *
 * This example also has a very basic watch-only wallet. You can add addresses to the wallet
 * by clicking the "Add address" button, and then entering the address in the text field.
 * You can also add a random address to the wallet by clicking the "Add random address" button.
 * This will generate a new random address, and add it to the wallet. The wallet is just a
 * list of addresses, and is not persisted anywhere. If you refresh the page, the wallet will
 * be empty again.
 *
 * This example purposely does not use any frameworks, and is written in plain JavaScript. So
 * it should be easy to understand, and modify. If you want to use this in production, you
 * should probably use a framework like React, Vue, or Angular. The code is also super commented,
 * so it should be easy to understand what is going on even if you are not familiar with JavaScript
 * or Utreexo.
 */

// Our Rust library is compiled to WebAssembly, and is exposed as a NPM package. To any
// Wasm exports, you need to import the package, and then call the init function. This
// will return a promise that resolves when the library is ready to be used. You can then
// import the FlorestaChain class, and use it to interact with the chain.
import init, { FlorestaChain } from './pkg/example_libfloresta.js';

// Last tip holds the tip that is currently displayed in the UI. We only want to update
// the UI when the tip changes, to avoid unnecessary re-renders.
// FlorestaChain is the main component that we use to interact with the chain. It is
// initialized in the run function, and is used to add blocks to the chain, and to
// get information about the chain.
let florestaChain, last_tip = 0;

// Download blocks and add them to the chain. This is a very naive implementation
// that just downloads the next block in the chain, and assumes that the chain
// is valid. If we get some invalid blocks, we'll just halt and cry.
async function update_tip() {
    let height = florestaChain.height + 1;
    while (true) {
        // Download the next block from the REST API. See the README for more info
        // about the API and the response format.
        const res = await fetch('http://localhost:8080/block/$'.replace("$", height), {
            mode: 'cors',
            method: 'GET',
            headers: {
                'Access-Control-Allow-Origin': '*'
            }
        });
        const block = await res.json();
        // If the block is empty, we have reached the tip of the chain (or the API is down)
        if (!block.data) {
            break;
        }
        console.log("adding block", block.data.block, height);
        florestaChain.accept_block(JSON.stringify(block.data));
        ++height;
    }
}
// Creates a new random address and adds it to the wallet
// This will send a transaction to the address, and will be shown in the UI
// once it is mined
function add_random_address_to_wallet() {
    const wallet = florestaChain.get_random_address();
    const wallet_field = document.getElementById('wallet');
    wallet_field.value = wallet;
    alert("Address " + wallet + " added to wallet");
    add_address_to_wallet();
}
// Add a user provided address to the wallet
function add_address_to_wallet() {
    const wallet = document.getElementById('wallet');
    florestaChain.add_address(wallet.value);

    wallet.value = "";
}
// Called when the user clicks the "Start" button
async function sync_loop() {
    await update_tip()
    florestaChain.toggle_ibd(); // We are done syncing, so we can tell our chain we are done
    update_ui();
    document.getElementById('start').disabled = true;
}

// Called when the page is loaded, to initialize the chain, and set up the UI
(async function run() {
    await init();
    // This is how you create a new chain from scratch, without any blocks
    florestaChain = new FlorestaChain();
    console.log("Loaded chain at height", florestaChain.height);
    // Set up the UI
    update_ui();
    // Set event handlers for buttons
    start.onclick = () => sync_loop();
    add_wallet.onclick = () => add_address_to_wallet();
    random_wallet.onclick = () => add_random_address_to_wallet();
    document.getElementById('start').disabled = false;
})().catch(console.error);

// Update the UI every few seconds
setInterval(() => {
    update_ui();
    // The user didn't click "Start", so we don't want to sync
    if (florestaChain.ibd) {
        return;
    }
    update_tip(); // If a new block has been added, download it
}, 5000);

function update_ui() {
    // Only update the UI if the tip has changed
    if (florestaChain.tip === last_tip) {
        return;
    }
    last_tip = florestaChain.tip;
    // Render the chain info in the UI. Not the most elegant way to do it, but it works
    const paragaph = document.getElementById('height');
    paragaph.innerHTML = `<strong>Best block</strong>: ${florestaChain.tip} <br>
                              <strong>Height</strong>: ${florestaChain.height} <br>
                              <strong>network</strong>: ${florestaChain.network} <br>
                              <strong>ibd</strong>: ${florestaChain.ibd} <br>
                              <strong>difficulty</strong>: ${florestaChain.difficulty} <br>
                              <strong>target</strong>: ${florestaChain.target} <br>
                            `;
    // If we have any transactions, render them in the UI
    const our_txs = document.getElementById('our_txs');
    our_txs.innerHTML = `Our txs: ${florestaChain.our_txs}`;
}