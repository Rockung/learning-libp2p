use async_std::io;
use futures::{prelude::*, select};
use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{
  Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity,
  ValidationMode,
};
use libp2p::Multiaddr;
use libp2p::{gossipsub, identity, swarm::SwarmEvent, NetworkBehaviour, PeerId, Swarm};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;

// In the first terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-dail
// ```
//
// In the second terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-dail -- /ip4/127.0.0.1/tcp/xxxxx
// ```
//
// In the third terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-dail -- /ip4/127.0.0.1/tcp/xxxxx
// ```

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // Create a random PeerId
  let local_key = identity::Keypair::generate_ed25519();
  let local_peer_id = PeerId::from(local_key.public());
  println!("Local peer id: {}", local_peer_id);

  // Set up an encrypted DNS-enabled TCP Transport over the Mplex protocol.
  let transport = libp2p::development_transport(local_key.clone()).await?;

  // We create a custom network behaviour that combines Gossipsub and Mdns.
  #[derive(NetworkBehaviour)]
  #[behaviour(out_event = "MyBehaviourEvent")]
  struct MyBehaviour {
    gossipsub: Gossipsub,
  }

  #[allow(clippy::large_enum_variant)]
  #[derive(Debug)]
  enum MyBehaviourEvent {
    Gossipsub(GossipsubEvent),
  }

  impl From<GossipsubEvent> for MyBehaviourEvent {
    fn from(v: GossipsubEvent) -> Self {
      Self::Gossipsub(v)
    }
  }

  // To content-address message, we can take the hash of message and use it as an ID.
  let message_id_fn = |message: &GossipsubMessage| {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    MessageId::from(s.finish().to_string())
  };

  // Set a custom gossipsub configuration
  let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
    .heartbeat_interval(Duration::from_secs(10))
    .validation_mode(ValidationMode::Strict)
    .message_id_fn(message_id_fn)
    .build()
    .expect("Valid config");

  // build a gossipsub network behaviour
  let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
    .expect("Correct configuration");

  // Create a Gossipsub topic
  let topic = Topic::new("test-net");

  // subscribes to our topic
  gossipsub.subscribe(&topic)?;

  // Create a Swarm to manage peers and events
  let mut swarm = {
    let behaviour = MyBehaviour { gossipsub };
    Swarm::new(transport, behaviour, local_peer_id)
  };

  // Listen on all interfaces and whatever port the OS assigns
  swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

  if let Some(to_dial) = std::env::args().nth(1) {
    let address: Multiaddr = to_dial.parse().expect("User to provide valid address.");
    match swarm.dial(address.clone()) {
      Ok(_) => println!("Dialed {:?}", address),
      Err(e) => println!("Dial {:?} failed: {:?}", address, e),
    };
  }

  // Read full lines from stdin
  let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
  println!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");

  // Kick it off
  loop {
    select! {
        line = stdin.select_next_some() => {
          if let Err(e) = swarm
            .behaviour_mut().gossipsub
            .publish(topic.clone(), line.expect("Stdin not to close").as_bytes()) {
              println!("Publish error: {:?}", e);
          }
        },
        event = swarm.select_next_some() => match event {
          SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(GossipsubEvent::Message {
              propagation_source: peer_id,
              message_id: id,
              message,
          })) =>  {
            println!(
              "Got message: '{}' with id: {id} from peer: {peer_id}",
              String::from_utf8_lossy(&message.data),
            );
          }
          SwarmEvent::NewListenAddr { address, .. } => {
              println!("Listening on {:?}", address);
          }
          _ => {}
        }
    }
  }
}
