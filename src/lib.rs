// SPDX-License-Identifier: Apache-2.0
#![allow(unused)]

use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::format,
    str::FromStr,
    sync::Arc,
};

use bitcoin::{
    consensus::{self, deserialize},
    hashes::{sha256, Hash},
    network::utreexo::CompactLeafData,
    Address, Block, BlockHash, BlockHeader, OutPoint, PrivateKey, Script, Transaction, TxOut,
};
use floresta_chain::{
    proof_util::{self, reconstruct_leaf_data},
    pruned_utreexo::error::DatabaseError,
    pruned_utreexo::{
        chain_state::ChainState, chain_state_builder::ChainStateBuilder, BlockchainInterface,
        UpdatableChainstate,
    },
    ChainStore, KvChainStore, Network,
};
use js_sys::Array;
use rustreexo::accumulator::{node_hash::NodeHash, proof::Proof};
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
    wallet: Wallet,
}
#[wasm_bindgen]
#[derive(Default, Debug)]
/// A super simple key value ChainStore using a HashMap. This is just for testing
/// purposes, the contents will be lost when the wasm instance is destroyed. You should
/// use a proper Wasm database implementation for production.
pub struct WasmStore {
    store: RefCell<HashMap<String, String>>,
}
#[wasm_bindgen]
#[derive(Default, Debug, Clone)]
pub struct Wallet {
    address_set: RefCell<HashSet<Script>>,
    transaction_list: RefCell<Vec<Transaction>>,
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
        let header = self
            .store
            .borrow()
            .get(&block_hash.to_string())
            .map(|s| hex::decode(s).unwrap())
            .map(|s| consensus::deserialize(&s).unwrap());
        Ok(header)
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
        let key = format!("index{height}");
        Ok(self
            .store
            .borrow()
            .get(&key)
            .map(|s| BlockHash::from_str(s).unwrap()))
    }

    fn flush(&self) -> Result<(), Error> {
        Ok(())
    }

    fn update_block_index(&self, height: u32, hash: BlockHash) -> Result<(), Error> {
        self.store
            .borrow_mut()
            .insert(format!("index{height}").into(), format!("{hash}"));
        Ok(())
    }
}
#[wasm_bindgen]
impl FlorestaChain {
    /// Creates a new FlorestaChain object. This should be used with new FlorestaChain()
    #[wasm_bindgen(constructor)]
    pub unsafe fn new() -> Self {
        let chain_state = ChainState::new(WasmStore::default(), Network::Regtest, None);
        let mut wallet = Wallet::default();
        let address = Address::from_str("bcrt1q9t6g0l36wgk454masqey03npa6esn370g4wuc9");
        wallet
            .address_set
            .borrow_mut()
            .insert(address.unwrap().script_pubkey());
        Self {
            chain_state,
            wallet,
        }
    }
    /// Add a new address to the wallet. This will be used to filter transactions.
    pub unsafe fn add_address(&self, addr: String) -> Result<(), String> {
        let address = Address::from_str(&addr).map_err(|_| "Invalid address")?;
        self.wallet
            .address_set
            .borrow_mut()
            .insert(address.script_pubkey().clone());
        Ok(())
    }
    /// Builds a chain from the given roots and tip. This is used to initialize the chain from
    /// a trusted source.
    pub unsafe fn build_chain_from(
        leaves: u64,
        roots: Array,
        tip: String,
        height: u32,
        header: String,
    ) -> Result<FlorestaChain, String> {
        let roots = roots
            .into_iter()
            .map(|x| x.as_string().unwrap().parse().unwrap())
            .collect();
        let header: BlockHeader = deserialize(&hex::decode(header).unwrap()).unwrap();

        let chain_state = ChainStateBuilder::new()
            .with_tip((tip.parse().unwrap(), height), header)
            .assume_utreexo(rustreexo::accumulator::stump::Stump { leaves, roots })
            .with_chainstore(WasmStore::default())
            .with_chain_params(Network::Regtest.into())
            .build()
            .map_err(|e| format!("{:?}", e))?;

        Ok(Self {
            chain_state,
            wallet: Wallet::default(),
        })
    }
    /// Returns the current height of the chain
    #[wasm_bindgen(getter, js_name = "height")]
    pub unsafe fn show_height(&self) -> u32 {
        self.chain_state.get_height().unwrap()
    }
    /// Whether the chain is currently in IBD (Initial Block Download) mode. This is true when the
    /// chain is still syncing with the network.
    #[wasm_bindgen(getter, js_name = "ibd")]
    pub unsafe fn show_ibd(&self) -> bool {
        self.chain_state.is_in_idb()
    }
    /// A string representing the network we are on. This is always "Regtest" for now
    #[wasm_bindgen(getter, js_name = "network")]
    pub unsafe fn show_network(&self) -> String {
        "Regtest".into()
    }
    /// Returns the current difficulty of the last block. This is a number that represents the
    /// amount of hashes that must be computed to find a valid block, on average. The returned value
    /// is a multiple of the minimum difficulty, which is different for each network.
    #[wasm_bindgen(getter, js_name = "difficulty")]
    pub unsafe fn show_difficulty(&self) -> u64 {
        let block = self.chain_state.get_best_block().unwrap();
        let header = self.chain_state.get_block_header(&block.1).unwrap();
        header.difficulty(bitcoin::Network::Regtest)
    }
    // The target is the uint256 number that sets the difficulty of the block. A valid solution
    // must be less than the target
    #[wasm_bindgen(getter, js_name = "target")]
    pub unsafe fn show_target(&self) -> String {
        let block = self.chain_state.get_best_block().unwrap();
        let header = self.chain_state.get_block_header(&block.1).unwrap();
        header.target().to_string()
    }

    /// Returns the best block hash
    #[wasm_bindgen(getter, js_name = "tip")]
    pub unsafe fn return_tip(&self) -> String {
        self.chain_state.get_best_block().unwrap().1.to_string()
    }
    /// Returns a random address. You shouldn't use this for anything other than testing
    pub unsafe fn get_random_address(&self) -> Result<String, String> {
        let mut key = [0u8; 32];
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let rand = getrandom::getrandom(&mut key).expect("Can't sample random bytes");
        let key = PrivateKey::from_slice(&key, bitcoin::Network::Regtest).unwrap();
        let pk = key.public_key(&secp);
        let address = Address::p2wpkh(&pk, bitcoin::Network::Regtest).map_err(|e| e.to_string())?;
        Ok(address.to_string())
    }
    #[wasm_bindgen(getter, js_name = "our_txs")]
    pub unsafe fn get_our_transactions(&self) -> String {
        self.wallet
            .transaction_list
            .borrow()
            .iter()
            .map(|tx| tx.txid().to_string())
            .reduce(|a, b| format!("{}\n {}", a, b))
            .unwrap_or("".into())
    }

    /// Accepts a new block to our chain. Validates the block and connects it to the chain
    /// if it is valid. Returns an error if the block is invalid.
    pub unsafe fn accept_block(&mut self, block: String) -> Result<(), String> {
        let block: WasmBlock = serde_json::from_str(&block).map_err(|e| e.to_string())?;

        let mut leaf_data = block.leaf_data;
        let mut proof = block.proof.into();
        self.chain_state
            .accept_header(block.block.header)
            .map_err(|e| format!("Accept header: {e:?}"))?;
        let (del_hashes, inputs) = self
            .process_proof(leaf_data, &block.block.txdata, &block.block.block_hash())
            .map_err(|e| format!("Process Proof: {e:?}"))?;
        let our_transactions = block
            .block
            .txdata
            .iter()
            .filter(|tx| {
                tx.output.iter().any(|output| {
                    self.wallet
                        .address_set
                        .borrow()
                        .contains(&output.script_pubkey)
                })
            })
            .cloned()
            .collect::<Vec<Transaction>>();

        self.wallet
            .transaction_list
            .borrow_mut()
            .extend(our_transactions);

        self.chain_state
            .connect_block(&block.block, proof, inputs, del_hashes)
            .map_err(|e| format!("Connect Block: {e:?}"))?;
        Ok(())
    }
    fn process_proof(
        &self,
        leaves: Vec<CompLeafData>,
        transactions: &[Transaction],
        block_hash: &BlockHash,
    ) -> anyhow::Result<(Vec<sha256::Hash>, HashMap<OutPoint, TxOut>)> {
        let mut leaves_iter = leaves.iter().cloned();
        let mut tx_iter = transactions.iter();

        let mut inputs = HashMap::new();
        tx_iter.next(); // Skip coinbase
        let mut hashes = vec![];
        for tx in tx_iter {
            let txid = tx.txid();
            for (vout, out) in tx.output.iter().enumerate() {
                inputs.insert(
                    OutPoint {
                        txid,
                        vout: vout as u32,
                    },
                    out.clone(),
                );
            }

            for input in tx.input.iter() {
                if !inputs.contains_key(&input.previous_output) {
                    if let Some(leaf) = leaves_iter.next() {
                        let height = leaf.header_code >> 1;
                        let hash = self
                            .chain_state
                            .get_block_hash(height)
                            .map_err(|e| anyhow::anyhow!("Failed to get block hash: {:?}", e))?;
                        let leaf = proof_util::reconstruct_leaf_data(&leaf.into(), input, hash)
                            .expect("Invalid proof");
                        // FIXME: Bring this back after finding wat the frick is going on with
                        // the bridge
                        // hashes.push(leaf._get_leaf_hashes());
                        inputs.insert(leaf.prevout, leaf.utxo);
                    }
                }
            }
        }
        Ok((hashes, inputs))
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
    proof: JsonProof,
    leaf_data: Vec<CompLeafData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompLeafData {
    /// Header code tells the height of creating for this UTXO and whether it's a coinbase
    pub header_code: u32,
    /// The amount locked in this UTXO
    pub amount: u64,
    /// The type of the locking script for this UTXO
    pub spk_ty: ScriptPubkeyType,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum ScriptPubkeyType {
    /// An non-specified type, in this case the script is just copied over
    Other(Box<[u8]>),
    /// p2pkh
    PubKeyHash,
    /// p2wsh
    WitnessV0PubKeyHash,
    /// p2sh
    ScriptHash,
    /// p2wsh
    WitnessV0ScriptHash,
}

#[derive(Clone, Serialize, Deserialize)]
struct JsonProof {
    targets: Vec<u64>,
    hashes: Vec<String>,
}

impl From<JsonProof> for Proof {
    fn from(json_proof: JsonProof) -> Self {
        let mut targets = Vec::new();
        let mut hashes = Vec::new();
        for target in json_proof.targets {
            targets.push(target);
        }
        for hash in json_proof.hashes {
            hashes.push(hash.parse().unwrap());
        }
        Proof { targets, hashes }
    }
}

impl From<CompLeafData> for CompactLeafData {
    fn from(leaf: CompLeafData) -> Self {
        let spk_ty: bitcoin::network::utreexo::ScriptPubkeyType = match leaf.spk_ty {
            ScriptPubkeyType::Other(script) => {
                bitcoin::network::utreexo::ScriptPubkeyType::Other(script)
            }
            ScriptPubkeyType::PubKeyHash => bitcoin::network::utreexo::ScriptPubkeyType::PubKeyHash,
            ScriptPubkeyType::WitnessV0PubKeyHash => {
                bitcoin::network::utreexo::ScriptPubkeyType::WitnessV0PubKeyHash
            }
            ScriptPubkeyType::ScriptHash => bitcoin::network::utreexo::ScriptPubkeyType::ScriptHash,
            ScriptPubkeyType::WitnessV0ScriptHash => {
                bitcoin::network::utreexo::ScriptPubkeyType::WitnessV0ScriptHash
            }
        };

        Self {
            header_code: leaf.header_code,
            amount: leaf.amount,
            spk_ty,
        }
    }
}
/// An alternative panic handler
#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}
