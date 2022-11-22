import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { bootstrap } from '@libp2p/bootstrap'
import bootstrapers from './bootstrappers.js'

async function main() {
  const node = await createLibp2p({
    addresses: {
      listen: ['/ip4/0.0.0.0/tcp/0']
    },
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [
      bootstrap({
        interval: 60e3, // 60 seconds
        list: bootstrapers
      })
    ]
  })

  node.connectionManager.addEventListener('peer:connect', (evt) => {
    const connection = evt.detail
    console.log('Connection established to:', connection.remotePeer.toString())	// Emitted when a peer has been found
  })

  node.addEventListener('peer:discovery', (evt) => {
    const peer = evt.detail
    // No need to dial, autoDial is on
    console.log('Discovered:', peer.id.toString())
  })

  await node.start()
}

main();
