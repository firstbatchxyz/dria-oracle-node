/**
 * A helper script to print the content of an Arweave transaction.
 *
 * Usage:
 *
 * ```sh
 * # calldata as-is
 * bun run ./misc/arweave.js 0x7b2261727765617665223a224d49555775656361634b417a62755442335a6a57613463784e6461774d71435a704550694f71675a625a63227d
 *
 * # as an object (with escaped quotes)
 * bun run ./misc/arweave.js "{\"arweave\":\"MIUWuecacKAzbuTB3ZjWa4cxNdawMqCZpEPiOqgZbZc\"}"
 *
 * # base64 txid
 * bun run ./misc/arweave.js MIUWuecacKAzbuTB3ZjWa4cxNdawMqCZpEPiOqgZbZc
 * ```
 *
 * Can be piped to `pbcopy` on macOS to copy the output to clipboard.
 */

// parse input
let input = process.argv[2];
if (!input) {
  console.error("No input provided.");
  return;
}

let arweaveTxId;
if (input.startsWith("0x")) {
  // if it starts with 0x, we assume its all hex
  arweaveTxId = JSON.parse(
    Buffer.from(input.slice(2), "hex").toString()
  ).arweave;
} else if (input.startsWith("{")) {
  // if it starts with {, we assume its a JSON string
  console.log("input", input);
  arweaveTxId = JSON.parse(input).arweave;
} else {
  // otherwise, we assume its a base64 txid
  arweaveTxId = input;
}

// construct the URL
// download the actual response from Arweave
const url = `https://arweave.net/${arweaveTxId}`;
const res = await fetch(url);
console.log(await res.text());
