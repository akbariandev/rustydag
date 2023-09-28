use super::{node::Node, block::Block};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;
use crate::chain::block::MlBlock;

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static ML_BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("ml_blocks"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub ml_blocks: Vec<MlBlock>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String,
}

pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: Node,
}

impl AppBehaviour {
    pub async fn new(
        app: Node,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(ML_BLOCK_TOPIC.clone());

        behaviour
    }
}

// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));
                    resp.ml_blocks.iter().for_each(|r| info!("{:?}", r));

                    self.app.blocks = self.app.choose_general_chain(self.app.blocks.clone(), resp.blocks);
                    self.app.ml_blocks = self.app.choose_ml_chain(self.app.ml_blocks.clone(), resp.ml_blocks);
                }
            } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                info!("sending local chain to {}", msg.source.to_string());
                let peer_id = resp.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(),
                        ml_blocks: self.app.ml_blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                info!("received new general block from {}", msg.source.to_string());
                self.app.try_add_general_block(block);
            }else if let Ok(block) = serde_json::from_slice::<MlBlock>(&msg.data) {
                info!("received new ML block from {}", msg.source.to_string());
                self.app.try_add_ml_block(block);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_general_chain(swarm: &Swarm<AppBehaviour>) {
    info!("Local General Chain: :");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}

pub fn handle_print_ml_chain(swarm: &Swarm<AppBehaviour>) {
    info!("Local ML chain: ");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.ml_blocks).expect("can jsonify ml blocks");
    info!("{}", pretty_json);
}

pub fn add_general_block(swarm: &mut Swarm<AppBehaviour>, data: String) {
    let behaviour = swarm.behaviour_mut();
    let latest_block = behaviour
        .app
        .blocks
        .last()
        .expect("there is at least one block");
    let block = Block::new_general_block(
        latest_block.id + 1,
        latest_block.hash.clone(),
        data.to_owned(),
    );
    let json = serde_json::to_string(&block).expect("can jsonify request");
    behaviour.app.blocks.push(block);
    info!("broadcasting new block");
    behaviour
        .floodsub
        .publish(BLOCK_TOPIC.clone(), json.as_bytes());
}

pub fn add_ml_block(swarm: &mut Swarm<AppBehaviour>,  data: String) {
    let behaviour = swarm.behaviour_mut();
    let latest_block = behaviour
        .app
        .ml_blocks
        .last()
        .expect("there is at least one block");
    let block = MlBlock::new_ml_block(
        latest_block.id + 1,
        latest_block.hash.clone(),
        data.to_owned(),
    );
    let json = serde_json::to_string(&block).expect("can jsonify request");
    behaviour.app.ml_blocks.push(block);
    info!("broadcasting new ML block");
    behaviour
        .floodsub
        .publish(ML_BLOCK_TOPIC.clone(), json.as_bytes());
}

