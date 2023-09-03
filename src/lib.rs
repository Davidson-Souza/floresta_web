// SPDX-License-Identifier: MIT

use bitcoin::{
    consensus::{self, deserialize},
    hashes::{sha256, Hash},
    network::utreexo::CompactLeafData,
    util::uint::Uint256,
    Address, Block, BlockHash, BlockHeader, OutPoint, PrivateKey, Script, Transaction, TxOut,
};
use floresta_chain::{
    proof_util,
    pruned_utreexo::{
        chain_state::ChainState, chain_state_builder::ChainStateBuilder, BlockchainInterface,
        UpdatableChainstate,
    },
    pruned_utreexo::{error::DatabaseError, ChainStore},
    Network,
};
use rustreexo::accumulator::proof::Proof;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    str::FromStr,
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: String);
}

#[derive(Debug)]
/// The error returned by our database. Empty for now.
pub struct Error;
impl DatabaseError for Error {}
#[wasm_bindgen]
/// A wrapper around a the chain struct.
pub struct FlorestaChain {
    chain_state: ChainState<WasmStore>,
    hashes: Vec<u8>,
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
        let chain_state = ChainState::new(WasmStore::default(), Network::Signet, None);
        let wallet = Wallet::default();
        Self {
            chain_state,
            wallet,
            hashes: Vec::new(),
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
        tip: String,
        height: u32,
        header: String,
    ) -> Result<FlorestaChain, String> {
        // let roots = roots
        //     .into_iter()
        //     .map(|x| x.as_string().unwrap().parse().unwrap())
        //     .collect();
        let leaves = 3589788;
        let roots = [
            [
                125_u8, 164, 105, 220, 149, 130, 211, 189, 195, 241, 42, 166, 124, 109, 143, 231,
                66, 116, 44, 102, 182, 140, 175, 152, 195, 63, 44, 56, 34, 5, 255, 116,
            ],
            [
                1, 67, 238, 54, 53, 30, 130, 100, 184, 211, 253, 100, 228, 90, 19, 118, 187, 212,
                89, 240, 173, 210, 49, 245, 59, 248, 23, 225, 101, 183, 197, 205,
            ],
            [
                140, 164, 48, 221, 81, 98, 92, 204, 237, 93, 223, 31, 206, 233, 165, 235, 68, 80,
                150, 195, 194, 249, 102, 75, 215, 241, 79, 123, 80, 151, 226, 152,
            ],
            [
                253, 169, 247, 218, 192, 182, 116, 7, 69, 248, 218, 57, 72, 214, 124, 228, 148,
                110, 198, 98, 83, 50, 251, 172, 183, 236, 220, 12, 67, 108, 4, 126,
            ],
            [
                55, 235, 54, 1, 65, 192, 18, 60, 196, 66, 122, 160, 171, 47, 250, 208, 33, 182,
                224, 54, 129, 126, 25, 137, 109, 195, 96, 7, 98, 121, 111, 193,
            ],
            [
                7, 8, 193, 215, 236, 171, 71, 178, 73, 249, 185, 93, 215, 77, 196, 8, 131, 131, 40,
                129, 199, 52, 215, 212, 155, 116, 21, 20, 163, 250, 13, 248,
            ],
            [
                78, 204, 53, 177, 253, 216, 178, 84, 183, 19, 75, 225, 36, 234, 85, 194, 13, 202,
                144, 183, 6, 18, 227, 33, 54, 197, 49, 39, 100, 183, 87, 195,
            ],
            [
                86, 155, 134, 165, 70, 86, 55, 121, 0, 240, 230, 199, 115, 151, 216, 161, 0, 106,
                78, 80, 159, 216, 88, 1, 185, 42, 39, 184, 201, 165, 102, 253,
            ],
            [
                16, 185, 24, 187, 173, 239, 240, 149, 54, 203, 104, 236, 117, 87, 25, 114, 108,
                234, 225, 149, 152, 0, 181, 73, 219, 62, 99, 165, 16, 57, 210, 156,
            ],
            [
                97, 82, 82, 234, 178, 94, 99, 110, 150, 238, 211, 11, 51, 13, 192, 204, 91, 255,
                232, 172, 22, 63, 236, 224, 248, 220, 9, 93, 14, 139, 71, 250,
            ],
            [
                49, 117, 230, 111, 51, 175, 131, 200, 226, 153, 209, 85, 192, 203, 165, 66, 59, 16,
                185, 224, 191, 97, 240, 98, 70, 222, 33, 12, 9, 242, 8, 150,
            ],
            [
                99, 42, 17, 79, 148, 114, 95, 130, 116, 31, 102, 136, 239, 74, 118, 208, 151, 160,
                222, 132, 129, 12, 173, 226, 17, 163, 235, 94, 78, 32, 147, 209,
            ],
        ]
        .into_iter()
        .map(|x| x.into())
        .collect::<Vec<_>>();

        let header: BlockHeader = deserialize(&hex::decode(header).unwrap()).unwrap();

        let chain_state = ChainStateBuilder::new()
            .with_tip((tip.parse().unwrap(), height), header)
            .assume_utreexo(rustreexo::accumulator::stump::Stump { leaves, roots })
            .with_chainstore(WasmStore::default())
            .with_chain_params(Network::Signet.into())
            .build()
            .map_err(|e| format!("{:?}", e))?;
        let hashes = include_bytes!("../hashes.bin");

        Ok(Self {
            chain_state,
            wallet: Wallet::default(),
            hashes: hashes.to_vec(),
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
    /// A string representing the network we are on. This is always "Signet" for now
    #[wasm_bindgen(getter, js_name = "network")]
    pub unsafe fn show_network(&self) -> String {
        "Signet".into()
    }
    /// Returns the current difficulty of the last block. This is a number that represents the
    /// amount of hashes that must be computed to find a valid block, on average. The returned value
    /// is a multiple of the minimum difficulty, which is different for each network.
    #[wasm_bindgen(getter, js_name = "difficulty")]
    pub unsafe fn show_difficulty(&self) -> u64 {
        let block = self.chain_state.get_best_block().unwrap();
        let header = self.chain_state.get_block_header(&block.1).unwrap();
        (Uint256([
            0x0000000000000000,
            0x0000000000000000,
            0x0000000000000000,
            0x00000377ae000000,
        ]) / header.target())
        .low_u64()
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
        getrandom::getrandom(&mut key).expect("Can't sample random bytes");
        let key = PrivateKey::from_slice(&key, bitcoin::Network::Signet).unwrap();
        let pk = key.public_key(&secp);
        let address = Address::p2wpkh(&pk, bitcoin::Network::Signet).map_err(|e| e.to_string())?;
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

        let leaf_data = block.leaf_data;
        let proof: Proof = block.proof.into();
        self.chain_state
            .accept_header(block.block.header)
            .map_err(|e| format!("Accept header: {e:?}"))?;
        let (del_hashes, inputs) = self
            .process_proof(leaf_data, &block.block.txdata)
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
    fn get_block_hash(&mut self, height: u32) -> BlockHash {
        let offset = (height * 32) as usize;
        let hash = &self.hashes[offset..(offset + 32)];
        BlockHash::from_slice(&hash).unwrap()
    }
    fn process_proof(
        &mut self,
        leaves: Vec<CompLeafData>,
        transactions: &[Transaction],
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
                        let hash = match self.chain_state.get_block_hash(height) {
                            Err(_) => self.get_block_hash(height),
                            Ok(hash) => hash,
                        };
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
