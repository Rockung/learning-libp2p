import * as IPFS from "ipfs-core";

async function main() {
  const node = await IPFS.create();
  const version = await node.version();
  console.log("Version: ", version.version);

  const fileAdded = await node.add({
    path: "hello.txt",
    content: "Hello world 101",
  });

  console.log("Added file: ", fileAdded.path, fileAdded.cid);

  const decoder = new TextDecoder();
  let text = '';

  for await (const chunk of node.cat(fileAdded.cid)) {
    text += decoder.decode(chunk, { stream: true});
  }

  console.log("Added file contents: ", text);
}

main();
