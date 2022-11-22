import { createLibp2p } from 'libp2p'
import { webSockets } from '@libp2p/websockets'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'

async function main() {
  const node = await createLibp2p({
    addresses: {
      listen: ['/ip4/0.0.0.0/tcp/0/ws']
      // TODO check "What is next?" section
      // announce: ['/dns4/auto-relay.libp2p.io/tcp/443/wss/p2p/QmWDn2LY8nannvSWJzruUYoLZ4vV83vfCBwd8DipvdgQc3']
    },
    transports: [
      webSockets()
    ],
    connectionEncryption: [
      noise()
    ],
    streamMuxers: [
      mplex()
    ],
    relay: {
      enabled: true,
      hop: {
        enabled: true
      },
      advertise: {
        enabled: true,
      }
    }
  })

  await node.start()

  console.log(`Node started with id ${node.peerId.toString()}`)
  console.log('Listening on:')
  node.getMultiaddrs().forEach((ma) => console.log(ma.toString()))
}

main()
