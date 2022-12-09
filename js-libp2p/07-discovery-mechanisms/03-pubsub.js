import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { floodsub } from '@libp2p/floodsub'
import { bootstrap } from '@libp2p/bootstrap'
import { pubsubPeerDiscovery } from '@libp2p/pubsub-peer-discovery'

const createNode = async (bootstrappers) => {
    const node = await createLibp2p({
        addresses: {
            listen: ['/ip4/0.0.0.0/tcp/0']
        },
        transports: [tcp()],
        streamMuxers: [mplex()],
        connectionEncryption: [noise()],
        pubsub: floodsub(),
        peerDiscovery: [
            bootstrap({
                list: bootstrappers
            }),
            pubsubPeerDiscovery({
                interval: 1000
            })
        ]
    })

    return node
}

async function main() {
    const relay = await createLibp2p({
        addresses: {
            listen: [
                '/ip4/0.0.0.0/tcp/0'
            ]
        },
        transports: [tcp()],
        streamMuxers: [mplex()],
        connectionEncryption: [noise()],
        pubsub: floodsub(),
        peerDiscovery: [
            pubsubPeerDiscovery({
                interval: 1000
            })
        ],
        relay: {
            enabled: true, // Allows you to dial and accept relayed connections. Does not make you a relay.
            hop: {
                enabled: true // Allows you to be a relay for other peers
            }
        }
    })
    console.log(`libp2p relay starting with id: ${relay.peerId.toString()}`)
    await relay.start()

    const relayMultiaddrs = relay.getMultiaddrs().map((m) => m.toString())

    const [node1, node2] = await Promise.all([
        createNode(relayMultiaddrs),
        createNode(relayMultiaddrs)
    ])

    node1.addEventListener('peer:discovery', (evt) => {
        const peer = evt.detail
        console.log(`Peer ${node1.peerId.toString()} discovered: ${peer.id.toString()}`)
    })
    node2.addEventListener('peer:discovery', (evt) => {
        const peer = evt.detail
        console.log(`Peer ${node2.peerId.toString()} discovered: ${peer.id.toString()}`)
    })

    ;[node1, node2].forEach((node, index) => console.log(`Node ${index} starting with id: ${node.peerId.toString()}`))

    await Promise.all([
        node1.start(),
        node2.start()
    ])
}

main()