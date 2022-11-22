use async_std::io;
use futures::{
    prelude::{stream::StreamExt, *},
    select,
};
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    identity,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    swarm::SwarmEvent,
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // Set up an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::development_transport(local_key).await?;

    // Create a Floodsub topic
    let floodsub_topic = floodsub::Topic::new("chat");

    // We create a custom network behaviour that combines floodsub and mDNS.
    // Use the derive to generate delegating NetworkBehaviour impl.
    #[derive(NetworkBehaviour)]
    #[behaviour(out_event = "MyBehaviourEvent")]
    struct MyBehaviour {
        floodsub: Floodsub,
        mdns: Mdns,
    }

    #[allow(clippy::large_enum_variant)]
    #[derive(Debug)]
    enum MyBehaviourEvent {
        Floodsub(FloodsubEvent),
        Mdns(MdnsEvent),
    }

    impl From<MdnsEvent> for MyBehaviourEvent {
        fn from(v: MdnsEvent) -> Self {
            Self::Mdns(v)
        }
    }

    impl From<FloodsubEvent> for MyBehaviourEvent {
        fn from(v: FloodsubEvent) -> Self {
            Self::Floodsub(v)
        }
    }

    // Create a Swarm to manage peers and events
    let mut swarm = {
        let mdns = Mdns::new(MdnsConfig::default()).await?;
        let mut behaviour = MyBehaviour {
            floodsub: Floodsub::new(local_peer_id),
            mdns,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());
        Swarm::new(transport, behaviour, local_peer_id)
    };

    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("Dialed {:?}", to_dial)
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Kick it off
    loop {
        select! {
            line = stdin.select_next_some() => {
                swarm
                .behaviour_mut().floodsub
                .publish(floodsub_topic.clone(), line.expect("Stdin not to close").as_bytes());
            },
            // event = swarm.select_next_some() => match event {
            //     SwarmEvent::NewListenAddr { address, .. } => {
            //         println!("Listening on {:?}", address);
            //     }
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(
            //         FloodsubEvent::Message(message)
            //     )) => {
            //         println!(
            //             "Received: '{:?}' from {:?}",
            //             String::from_utf8_lossy(&message.data),
            //             message.source
            //         );
            //     }
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(
            //         MdnsEvent::Discovered(list)
            //     )) => {
            //         for (peer, _) in list {
            //             swarm
            //                 .behaviour_mut()
            //                 .floodsub
            //                 .add_node_to_partial_view(peer);
            //         }
            //     }
            //     SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(MdnsEvent::Expired(
            //         list
            //     ))) => {
            //         for (peer, _) in list {
            //             if !swarm.behaviour_mut().mdns.has_node(&peer) {
            //                 swarm
            //                     .behaviour_mut()
            //                     .floodsub
            //                     .remove_node_from_partial_view(&peer);
            //             }
            //         }
            //     },
            //     _ => {}
            // }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(MdnsEvent::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {}", peer_id);
                        swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(MdnsEvent::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {}", peer_id);
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(FloodsubEvent::Message(message))) =>  {
                    println!("Got message: '{}'", String::from_utf8_lossy(&message.data));
                },
                _ => {}
            }
        }
    }
}
