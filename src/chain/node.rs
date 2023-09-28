use chrono::prelude::*;
use log::{error, warn};
use sha2::{Digest, Sha256};
use crate::chain;
use crate::chain::block::MlBlock;
use super::{block::Block};

pub const GENERAL_DIFFICULTY_PREFIX: &str = "00";
pub const ML_DIFFICULTY_PREFIX: &str = "01";

pub struct Node {
    pub blocks: Vec<Block>,
    pub ml_blocks: Vec<MlBlock>,
}

impl Node {
    pub fn new() -> Self {
        Self { blocks: vec![],ml_blocks: vec![] }
    }

    pub fn general_genesis(&mut self) {
        let block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("general_genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(block);
    }

    pub fn ml_genesis(&mut self) {
        let block = MlBlock {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("ml_genesis!"),
            nonce: 1836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.ml_blocks.push(block);
    }

    pub fn try_add_general_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if self.is_general_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }

    pub fn try_add_ml_block(&mut self, block: MlBlock) {
        let latest_block = self.ml_blocks.last().expect("there is at least one block");
        if self.is_ml_block_valid(&block, latest_block) {
            self.ml_blocks.push(block);
        } else {
            error!("could not add ML block - invalid");
        }
    }

    pub fn is_general_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("block with id: {} has wrong previous hash", block.id);
            return false;
        } else if chain::block::hash_to_binary_representation(
            &hex::decode(&block.hash).expect("can decode from hex"),
        )
            .starts_with(GENERAL_DIFFICULTY_PREFIX)
        {
            warn!("block with id: {} has invalid difficulty", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!(
            "block with id: {} is not the next block after the latest: {}",
            block.id, previous_block.id
        );
            return false;
        } else if hex::encode(Node::calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
        )) != block.hash
        {
            warn!("block with id: {} has invalid hash", block.id);
            return false;
        }
        true
    }

    pub fn is_ml_block_valid(&self, block: &MlBlock, previous_block: &MlBlock) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("ml block with id: {} has wrong previous hash", block.id);
            return false;
        } else if chain::block::hash_to_binary_representation(
            &hex::decode(&block.hash).expect("can decode from hex"),
        )
            .starts_with(ML_DIFFICULTY_PREFIX)
        {
            warn!("ml block with id: {} has invalid difficulty", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!(
            "ml block with id: {} is not the next block after the latest: {}",
            block.id, previous_block.id
        );
            return false;
        } else if hex::encode(Node::calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
        )) != block.hash
        {
            warn!("ml block with id: {} has invalid hash", block.id);
            return false;
        }
        true
    }

    pub fn is_general_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_general_block_valid(second, first) {
                return false;
            }
        }
        true
    }

    pub fn is_ml_chain_valid(&self, chain: &[MlBlock]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_ml_block_valid(second, first) {
                return false;
            }
        }
        true
    }

    pub fn choose_general_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        let is_local_valid = self.is_general_chain_valid(&local);
        let is_remote_valid = self.is_general_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote general chains are both invalid");
        }
    }

    pub fn choose_ml_chain(&mut self, local: Vec<MlBlock>, remote: Vec<MlBlock>) -> Vec<MlBlock> {
        let is_local_valid = self.is_ml_chain_valid(&local);
        let is_remote_valid = self.is_ml_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote ML chains are both invalid");
        }
    }

    pub fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
        let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });
        let mut hasher = Sha256::new();
        hasher.update(data.to_string().as_bytes());
        hasher.finalize().as_slice().to_owned()
    }
}

