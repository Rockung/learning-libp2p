use async_std::io;
use futures::{prelude::*, select};
use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{
  Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity,
  ValidationMode,
};
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaConfig, KademliaEvent};
use libp2p::Multiaddr;
use libp2p::{gossipsub, identity, swarm::SwarmEvent, NetworkBehaviour, PeerId, Swarm};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::Duration;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
struct MyBehaviour {
  kademlia: Kademlia<MemoryStore>,
  gossipsub: Gossipsub,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum MyBehaviourEvent {
  Kademlia(KademliaEvent),
  Gossipsub(GossipsubEvent),
}

impl From<KademliaEvent> for MyBehaviourEvent {
  fn from(v: KademliaEvent) -> Self {
    Self::Kademlia(v)
  }
}

impl From<GossipsubEvent> for MyBehaviourEvent {
  fn from(v: GossipsubEvent) -> Self {
    Self::Gossipsub(v)
  }
}

// In the first terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-kad-args
// ```
//
// In the second terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-kad-args -- xxxxxxxx(peer-id) /ip4/127.0.0.1/tcp/xxxxx(addr)
// ```
//
// In the third terminal window, run
// ```sh
// cargo run --example 03-chat-gossip-kad-args -- xxxxxxxx(peer-id) /ip4/127.0.0.1/tcp/xxxxx(addr)
// ```

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
  if std::env::var_os("RUST_LOG").is_none() {
    // std::env::set_var("RUST_LOG", "libp2p_demo=true");
  }

  tracing_subscriber::fmt::init();

  // Create a random PeerId
  let local_key = identity::Keypair::generate_ed25519();
  let local_peer_id = PeerId::from(local_key.public());
  println!("Local peer id: {}", local_peer_id);

  // Set up an encrypted DNS-enabled TCP Transport over the Mplex protocol.
  let transport = libp2p::development_transport(local_key.clone()).await?;

  let behaviour = {
    // Build a kademlia network behavior
    let mut cfg = KademliaConfig::default();
    cfg.set_query_timeout(Duration::from_secs(5 * 60));
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::with_config(local_peer_id, store, cfg);

    // Build a gossipsub network behaviour

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

    let gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
      .expect("Correct configuration");

    MyBehaviour {
      kademlia,
      gossipsub,
    }
  };

  // Create a Swarm to manage peers and events
  let mut swarm = { Swarm::new(transport, behaviour, local_peer_id) };
  // Listen on all interfaces and whatever port the OS assigns
  swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

  // Bootstrap !!!important!!!
  if let Some(peer_id) = std::env::args().nth(1) {
    if let Some(to_dial) = std::env::args().nth(2) {
      let address: Multiaddr = to_dial.parse().expect("User to provide valid address.");
      swarm
        .behaviour_mut()
        .kademlia
        .add_address(&PeerId::from_str(peer_id.as_str())?, address);
      swarm.behaviour_mut().kademlia.bootstrap()?;
    }
  }

  // Create a Gossipsub topic
  let topic = Topic::new("test-net");
  swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

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
          SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(event)) => {
            process_kad_events(&swarm.behaviour_mut().kademlia, event);
          }
          _ => {}
        }
    }
  }
}

#[macro_use]
extern crate tracing;

// modified from `rust-ipfs`
fn process_kad_events(kademlia: &Kademlia<MemoryStore>, event: KademliaEvent) {
  use libp2p::kad::{
    AddProviderError, AddProviderOk, BootstrapError, BootstrapOk, GetClosestPeersError,
    GetClosestPeersOk, GetProvidersError, GetProvidersOk, GetRecordError, GetRecordOk,
    KademliaEvent::*, PutRecordError, PutRecordOk, QueryResult::*,
  };

  match event {
    InboundRequest { request } => {
      trace!("kad: inbound {:?} request handled", request);
    }
    OutboundQueryCompleted { result, id, .. } => {
      // make sure the query is exhausted
      if kademlia.query(&id).is_none() {
        match result {
          // these subscriptions return actual values
          GetClosestPeers(_) | GetProviders(_) | GetRecord(_) => {}
          // we want to return specific errors for the following
          Bootstrap(Err(_)) | StartProviding(Err(_)) | PutRecord(Err(_)) => {}
          // and the rest can just return a general KadResult::Complete
          _ => {
            trace!("kad: query {:?} completed", id);
          }
        }
      }

      match result {
        Bootstrap(Ok(BootstrapOk {
          peer,
          num_remaining,
        })) => {
          debug!(
            "kad: bootstrapped with {}, {} peers remain",
            peer, num_remaining
          );
        }
        Bootstrap(Err(BootstrapError::Timeout { .. })) => {
          warn!("kad: timed out while trying to bootstrap");

          if kademlia.query(&id).is_none() {
            trace!("kad: timed out while trying to bootstrap: {:?}", id);
          }
        }
        GetClosestPeers(Ok(GetClosestPeersOk { key: _, peers: _ })) => {
          if kademlia.query(&id).is_none() {
            trace!("kad: got lots of peers");
          }
        }
        GetClosestPeers(Err(GetClosestPeersError::Timeout { key: _, peers: _ })) => {
          // don't mention the key here, as this is just the id of our node
          warn!("kad: timed out while trying to find all closest peers");
        }
        GetProviders(Ok(GetProvidersOk {
          key: _,
          providers: _,
          closest_peers: _,
        })) => {
          trace!("kad: got lots of providers");
        }
        GetProviders(Err(GetProvidersError::Timeout { key: _, .. })) => {
          trace!("timed out while trying to get providers for the given key");
        }
        StartProviding(Ok(AddProviderOk { key })) => {
          debug!("kad: providing {:?}", key);
        }
        StartProviding(Err(AddProviderError::Timeout { key: _ })) => {
          trace!("kad: timed out while trying to provide the record");
        }
        RepublishProvider(Ok(AddProviderOk { key })) => {
          debug!("kad: republished provider {:?}", key);
        }
        RepublishProvider(Err(AddProviderError::Timeout { key })) => {
          warn!(
            "kad: timed out while trying to republish provider {:?}",
            key
          );
        }
        GetRecord(Ok(GetRecordOk { records: _, .. })) => {
          trace!("kad: got lots of records");
        }
        GetRecord(Err(GetRecordError::NotFound {
          key,
          closest_peers: _,
        })) => {
          warn!("kad: couldn't find record {:?}", key);
        }
        GetRecord(Err(GetRecordError::QuorumFailed {
          key,
          records: _,
          quorum,
        })) => {
          warn!(
            "kad: quorum failed {:?} when trying to get key {:?}",
            quorum, key
          );
        }
        GetRecord(Err(GetRecordError::Timeout {
          key,
          records: _,
          quorum: _,
        })) => {
          warn!("kad: timed out while trying to get key {:?}", key);
        }
        PutRecord(Ok(PutRecordOk { key })) | RepublishRecord(Ok(PutRecordOk { key })) => {
          debug!("kad: successfully put record {:?}", key);
        }
        PutRecord(Err(PutRecordError::QuorumFailed {
          key,
          success: _,
          quorum,
        }))
        | RepublishRecord(Err(PutRecordError::QuorumFailed {
          key,
          success: _,
          quorum,
        })) => {
          warn!(
            "kad: quorum failed ({:?}) when trying to put record {:?}",
            quorum, key
          );
        }
        PutRecord(Err(PutRecordError::Timeout {
          key,
          success: _,
          quorum: _,
        })) => {
          warn!("kad: timed out while trying to put record {:?}", key);
        }
        RepublishRecord(Err(PutRecordError::Timeout {
          key,
          success: _,
          quorum: _,
        })) => {
          warn!("kad: timed out while trying to republish record {:?}", key);
        }
      }
    }
    RoutingUpdated {
      peer,
      is_new_peer: _,
      addresses,
      bucket_range: _,
      old_peer: _,
    } => {
      trace!("kad: routing updated; {}: {:?}", peer, addresses);
    }
    UnroutablePeer { peer } => {
      trace!("kad: peer {} is unroutable", peer);
    }
    RoutablePeer { peer, address } => {
      trace!("kad: peer {} ({}) is routable", peer, address);
    }
    PendingRoutablePeer { peer, address } => {
      trace!("kad: pending routable peer {} ({})", peer, address);
    }
  }
}
