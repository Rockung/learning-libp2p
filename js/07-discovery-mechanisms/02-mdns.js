import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { mdns } from '@libp2p/mdns'

const createNode = async () => {
    const node = await createLibp2p({
        addresses: {
            listen: ['/ip4/0.0.0.0/tcp/0']
        },
        transports: [
            tcp()
        ],
        streamMuxers: [
            mplex()
        ],
        connectionEncryption: [
            noise()
        ],
        peerDiscovery: [
            mdns({
                interval: 20e3
            })
        ]
    })

    return node
}

async function main() {
    const [node1, node2] = await Promise.all([
        createNode(),
        createNode()
    ])

    node1.connectionManager.addEventListener('peer:connect', (evt) => {
        const connection = evt.detail
        console.log('Connection established to:', connection.remotePeer.toString())	// Emitted when a peer has been found
    })
    node1.addEventListener('peer:discovery', (evt) => console.log('Discovered:', evt.detail.id.toString()))

    node2.connectionManager.addEventListener('peer:connect', (evt) => {
        const connection = evt.detail
        console.log('Connection established to:', connection.remotePeer.toString())	// Emitted when a peer has been found
    })
    node2.addEventListener('peer:discovery', (evt) => console.log('Discovered:', evt.detail.id.toString()))

    await Promise.all([
        node1.start(),
        node2.start()
    ])
}

main();
