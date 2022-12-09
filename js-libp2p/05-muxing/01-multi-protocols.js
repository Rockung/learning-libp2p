import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { pipe } from 'it-pipe'
import { fromString as uint8ArrayFromString } from 'uint8arrays/from-string'
import { toString as uint8ArrayToString } from 'uint8arrays/to-string'

const createNode = async () => {
    const node = await createLibp2p({
        addresses: {
            listen: ['/ip4/0.0.0.0/tcp/0']
        },
        transports: [tcp()],
        streamMuxers: [mplex()],
        connectionEncryption: [noise()]
    })

    await node.start()
    return node
}

async function main() {
    const [node1, node2] = await Promise.all([
        createNode(),
        createNode()
    ])

    // Add node's 2 data to the PeerStore
    await node1.peerStore.addressBook.set(node2.peerId, node2.getMultiaddrs())

    node2.handle(['/your-protocol/1.0.0', '/your-protocol/2.0.0'], ({ stream }) => {
        console.log("protocol: ", stream.stat.protocol)
        if (stream.stat.protocol === 'your-protocol/1.0.0') {
            // handle backwards compatibility
        }
        
        pipe(
            stream,
            async function (source) {
                for await (const msg of source) {
                    console.log(uint8ArrayToString(msg.subarray()))
                }
            }
        )
    })

    const stream = await node1.dialProtocol(node2.peerId, ['/your-protocol/2.0.0'])
    await pipe(
        [uint8ArrayFromString('my own protocol, wow!')],
        stream
    )
}

main()
