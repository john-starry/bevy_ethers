use bevy::prelude::*;
use bevy_ethers::{RpcUrl, WsRpcUrl, EthersPlugin, BlockEvent};

fn main() {
    let url = std::env::var("ETH_RPC_URL").expect("url env not exist");
    let ws_url = std::env::var("WS_ETH_RPC_URL").expect("ws url env not exist");
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(RpcUrl(url.to_owned()))
        .insert_resource(WsRpcUrl(ws_url.to_owned()))
        .add_plugin(EthersPlugin)
        .add_system(print_block)
        .run();
}
fn print_block(mut events: EventReader<BlockEvent>) {
    for e in events.iter() {
        println!("{:?}", e.0.hash);
    }
}