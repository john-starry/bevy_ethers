use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use bevy::prelude::*;
use crossbeam::channel::{bounded, Receiver, Sender};
use ethers::providers::{StreamExt, Ws};
use ethers::{
    providers::{Middleware, Provider},
    types::{Block, Transaction},
};
use tokio::runtime::Runtime;

pub struct EthersPlugin;

pub struct BlockEvent(pub Block<Transaction>);

impl Plugin for EthersPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BlockEvent>()
            // .add_startup_system(start)
            .add_startup_system(subscribe_block)
            .add_system(generate_block_event);
    }
}

// get latest block data
// thread -> get data

#[derive(Resource, Deref)]
struct LatestBlockReceiver(Receiver<BlockEvent>);

#[derive(Resource)]
pub struct RpcUrl(pub String);
#[derive(Resource)]
pub struct WsRpcUrl(pub String);

fn start(mut commands: Commands, rpc_url: Res<RpcUrl>) {
    let (tx, rx) = bounded::<BlockEvent>(10);
    let rt = Arc::new(Mutex::new(Runtime::new().unwrap()));
    let rt_clone = rt.clone();
    let url = rpc_url.0.clone();
    std::thread::spawn(move || loop {
        if let Ok(rt) = rt_clone.lock() {
            let jh = rt.spawn(get_latest_block(url.clone()));
            if let Ok(Ok(block)) = rt.block_on(jh) {
                tx.send(BlockEvent(block)).expect("send error");
            }
        }
    });
    commands.insert_resource(LatestBlockReceiver(rx));
}

async fn subscribe(url: String, tx: Sender<BlockEvent>) {
    let ws = Ws::connect(url).await.unwrap();
    let provider = Provider::new(ws).interval(Duration::from_millis(2000));
    let mut stream = provider.watch_blocks().await.unwrap();
    while let Some(block) = stream.next().await {
        let block = provider.get_block_with_txs(block).await.unwrap().unwrap();
        tx.send(BlockEvent(block)).unwrap();
    }
}

fn subscribe_block(mut commands: Commands, url: Res<WsRpcUrl>) {
    let url = url.0.clone();
    let (tx, rx) = bounded::<BlockEvent>(10);
    let rt = Runtime::new().expect("new runtime error");
    std::thread::spawn(move || {
        let jh = rt.spawn(subscribe(url, tx));
        rt.block_on(jh).expect("block on error");
    });
    commands.insert_resource(LatestBlockReceiver(rx));
}

async fn get_latest_block(url: String) -> Result<Block<Transaction>> {
    let provider = Provider::try_from(url)?;
    let number = provider.get_block_number().await?;
    let block = provider
        .get_block_with_txs(number)
        .await?
        .ok_or(anyhow!("nonblock"))?;
    Ok(block)
}

fn generate_block_event(receiver: Res<LatestBlockReceiver>, mut events: EventWriter<BlockEvent>) {
    for block in receiver.try_iter() {
        events.send(block);
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_get_block() {
        let rt = Runtime::new().unwrap();
        let url = "https://rpc.ankr.com/eth".to_owned();
        let jh = rt.spawn(get_latest_block(url));
        let block = rt.block_on(jh).unwrap().unwrap();
        assert!(block.hash.is_some())
    }

    #[test]
    fn test_subscribe() {
        let jh = std::thread::spawn(move || {
            let rt = Arc::new(Mutex::new(Runtime::new().unwrap()));
            let a = async {
                let url = std::env::var("WS_ETH_RPC_URL").expect("not env");
                let ws = Ws::connect(url).await.unwrap();
                let provider = Provider::new(ws).interval(Duration::from_millis(2000));
                let mut stream = provider.watch_blocks().await.unwrap().take(1);
                while let Some(block) = stream.next().await {
                    let block = provider.get_block_with_txs(block).await.unwrap().unwrap();
                    println!("{:?}", block.hash)
                    // tx.send(BlockEvent(block)).expect("send error");
                }
            };
            if let Ok(rt) = rt.clone().lock() {
                let handle = rt.spawn(a);
                rt.block_on(handle).unwrap();
            }
        });
        jh.join().unwrap();
    }
}
