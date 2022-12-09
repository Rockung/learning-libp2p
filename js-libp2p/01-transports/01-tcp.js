import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { noise } from '@chainsafe/libp2p-noise'

const createNode = async () => {
    const node = await createLibp2p({
        addresses: {
            // To signal the addresses we want to be available, we use
            // the multiaddr format, a self describable address
            listen: [
                '/ip4/0.0.0.0/tcp/0'
            ]
        },
        transports: [
            tcp()
        ],
        connectionEncryption: [
            noise()
        ]
    })

    await node.start()
    return node
}

async function main() {
    const node = await createNode()

    console.log('node has started (true/false):', node.isStarted())
    console.log('listening on:')
    node.getMultiaddrs().forEach((ma) => console.log(ma.toString()))
}

main()
