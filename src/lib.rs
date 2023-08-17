// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, fmt::format};

use bitcoin::{consensus, hashes::Hash, Block, BlockHash, BlockHeader};
use floresta_chain::{
    pruned_utreexo::error::DatabaseError,
    pruned_utreexo::{chain_state::ChainState, BlockchainInterface, UpdatableChainstate},
    ChainStore, KvChainStore, Network,
};
use rustreexo::accumulator::proof::Proof;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

#[derive(Debug)]
/// The error returned by our database. Empty for now.
pub struct Error;
impl DatabaseError for Error {}
#[wasm_bindgen]
/// A wrapper around a the chain struct.
pub struct FlorestaChain {
    chain_state: ChainState<WasmStore>,
}
#[wasm_bindgen]
#[derive(Default, Debug)]
/// A super simple key value ChainStore using a HashMap. This is just for testing
/// purposes, the contents will be lost when the wasm instance is destroyed. You should
/// use a proper Wasm database implementation for production.
pub struct WasmStore {
    store: RefCell<HashMap<String, String>>,
}

impl ChainStore for WasmStore {
    type Error = Error;
    fn save_roots(&self, roots: Vec<u8>) -> Result<(), Error> {
        self.store
            .borrow_mut()
            .insert("roots".into(), hex::encode(roots));
        Ok(())
    }

    fn load_roots(&self) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .store
            .borrow()
            .get("roots")
            .map(|s| hex::decode(s).unwrap())
            .map(|s| consensus::deserialize(&s).unwrap()))
    }

    fn load_height(&self) -> Result<Option<floresta_chain::BestChain>, Error> {
        Ok(self
            .store
            .borrow()
            .get("height")
            .map(|s| hex::decode(s).unwrap())
            .map(|s| consensus::deserialize(&s).unwrap()))
    }

    fn save_height(&self, height: &floresta_chain::BestChain) -> Result<(), Error> {
        self.store
            .borrow_mut()
            .insert("height".into(), format!("{height:?}"));
        Ok(())
    }

    fn get_header(
        &self,
        block_hash: &BlockHash,
    ) -> Result<Option<floresta_chain::DiskBlockHeader>, Error> {
        Ok(self
            .store
            .borrow()
            .get(&block_hash.to_string())
            .map(|s| hex::decode(s).unwrap())
            .map(|s| consensus::deserialize(&s).unwrap()))
    }

    fn save_header(&self, header: &floresta_chain::DiskBlockHeader) -> Result<(), Error> {
        let ser_header = consensus::serialize(&header);
        let ser_header = hex::encode(ser_header);
        self.store
            .borrow_mut()
            .insert(header.block_hash().to_string(), ser_header);

        Ok(())
    }

    fn get_block_hash(&self, height: u32) -> Result<Option<BlockHash>, Error> {
        Ok(self
            .store
            .borrow()
            .get(&height.to_string())
            .map(|s| hex::decode(s).unwrap())
            .map(|s| consensus::deserialize(&s).unwrap()))
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }

    fn update_block_index(&self, height: u32, hash: BlockHash) -> Result<(), Error> {
        self.store
            .borrow_mut()
            .insert("index".into(), format!("{height}{hash}"));
        Ok(())
    }
}
#[wasm_bindgen]
impl FlorestaChain {
    /// Creates a new FlorestaChain object. This should be used with new FlorestaChain()
    #[wasm_bindgen(constructor)]
    pub unsafe fn new() -> Self {
        let chain_state = ChainState::new(WasmStore::default(), Network::Signet, None);
        Self { chain_state }
    }
    /// Returns the current height of the chain
    #[wasm_bindgen(getter, js_name = "height")]
    pub unsafe fn show_height(&self) -> u32 {
        self.chain_state.get_height().unwrap()
    }
    #[wasm_bindgen(getter, js_name = "ibd")]
    pub unsafe fn show_ibd(&self) -> bool {
        self.chain_state.is_in_idb()
    }
    #[wasm_bindgen(getter, js_name = "network")]
    pub unsafe fn show_network(&self) -> String {
        "Signet".into()
    }
    #[wasm_bindgen(getter, js_name = "difficulty")]
    pub unsafe fn show_difficulty(&self) -> String {
        let block = self.chain_state.get_best_block().unwrap();
        let header = self.chain_state.get_block_header(&block.1).unwrap();
        header.difficulty(bitcoin::Network::Signet).to_string()
    }
    /// Returns the best block hash
    #[wasm_bindgen(getter, js_name = "tip")]
    pub unsafe fn return_tip(&self) -> String {
        self.chain_state.get_best_block().unwrap().1.to_string()
    }
    /// Accepts a new block to our chain. Validates the block and connects it to the chain
    /// if it is valid. Returns an error if the block is invalid.
    pub unsafe fn accept_block(&self, block: String) -> Result<(), String> {
        let block: WasmBlock = serde_json::from_str(&block).map_err(|e| e.to_string())?;
        self.chain_state
            .accept_header(block.block.header)
            .map_err(|e| format!("{e:?}"))?;
        self.chain_state
            .connect_block(&block.block, block.proof, HashMap::new(), vec![])
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
    pub unsafe fn toggle_ibd(&self) {
        self.chain_state.toggle_ibd(false);
    }
}
#[wasm_bindgen]
#[derive(Deserialize, Serialize)]
/// A block and a set of proof. Using this here because we still don't have serde for
/// UtreexoBlock in my rust-bitcoin fork. We pass this as a stringified json object
pub struct WasmBlock {
    block: Block,
    proof: Proof,
    leaf_data: Vec<Vec<u8>>,
}
