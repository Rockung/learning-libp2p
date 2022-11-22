use futures::StreamExt;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{identity, ping, Multiaddr, PeerId};
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // create a network identify for local node
    //   - via public/private key pair
    //   - derived from their public key
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // Transport
    //   - provide connection-oriented communication channels
    //   - upgrade on top of those like authentication and encryption protocols
    let transport = libp2p::development_transport(local_key).await?;

    // NetworkBehaviour
    //   - define what bytes to send on the network
    let behaviour = ping::Behaviour::new(ping::Config::new().with_keep_alive(true));

    // Swarm
    //   - connect transport and behaviour, allowing both to make progress
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    // listen on all interfaces and a random, OS-assigned port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // dial the peer identified by the multi-address given as the second
    // command-line argument, if any
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {}", addr);
    }

    // continuously poll the swarm
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {:?}", address),
            SwarmEvent::Behaviour(event) => println!("{:?}", event),
            _ => {}
        }
    }
}

// cargo run --example 01-ping
// cargo run --example 01-ping -- /ip4/127.0.0.1/tcp/xxxxx
