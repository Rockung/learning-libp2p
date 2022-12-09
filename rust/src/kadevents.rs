use libp2p::kad::{
  store::MemoryStore,
  AddProviderError, AddProviderOk, BootstrapError, BootstrapOk, GetClosestPeersError,
  GetClosestPeersOk, GetProvidersError, GetProvidersOk, GetRecordError, GetRecordOk, Kademlia,
  KademliaEvent::{self, *},
  PutRecordError, PutRecordOk,
  QueryResult::*,
};

// modified from `rust-ipfs`
pub fn process_kad_events(kademlia: &Kademlia<MemoryStore>, event: KademliaEvent) {
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
