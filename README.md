<p align="center">
  <img src="https://raw.githubusercontent.com/firstbatchxyz/.github/refs/heads/master/branding/dria-logo-square.svg" alt="logo" width="168">
</p>

<p align="center">
  <h1 align="center">
    Dria Oracle Node
  </h1>
  <p align="center">
    <i>Dria Oracle Node serves LLM workflow tasks directly from smart-contracts.</i>
  </p>
</p>

## Installation

Install Dria Oracle Node with:

```sh
cargo install --git https://github.com/firstbatchxyz/dria-oracle-node
```

## Setup

Create an `.env` file by copying `.env.example`. You have to fill the following variables:

- Get an RPC URL from a provider such as [Alchemy](https://www.alchemy.com/) or [Infura](https://www.infura.io/), and set it as `RPC_URL`.
- Provide an Ethereum wallet secret key to `SECRET_KEY`, make sure it has funds to pay for gas and tokens.

> [!NOTE]
>
> The contract addresses are determined with respect to the chain connected via RPC URL, but you can override it via `COORDINATOR_ADDRESS` environment variable.
> In any case, you should not need to do this.

> [!TIP]
>
> You can have multiple environment files, and specify them explicitly with the `-e` argument, e.g.
>
> ```sh
> # base sepolia
> dria-oracle -e ./.env.base-sepolia registrations
>
> # base mainnet
> dria-oracle -e ./.env.base-mainnet registrations
> ```

### Arweave

You can save gas costs using [Arweave](https://arweave.org/):

- Provide an Arweave wallet via `ARWEAVE_WALLET_PATH` variable so that you can use Arweave for large results. You can create one [here](https://arweave.app/).
- You can set `ARWEAVE_BYTE_LIMIT` to determine the byte length threshold, beyond which values are uploaded to Arweave. It defaults to 1024, so any data less than that many bytes will be written as-is.

If you omit Arweave, it will only use the client for downloading things from Arweave, but will never upload.

### LLM Providers

As for the LLM providers:

- If you are using Ollama, make sure it is running and the host & port are correct.
- If you are using OpenAI, provide the `OPENAI_API_KEY`.
- If you are using Gemini, provide the `GEMINI_API_KEY`.
- If you are using OpenRouter, provide the `OPENROUTER_API_KEY`.

## Usage

After installatioon, a binary called `dria-oracle` will be created. You can see the available commands with:

```sh
dria-oracle help
```

The CLI provides several methods to interact with the oracle contracts.

- [Registration](#registration)
- [Launching the Node](#launching-the-node)
- [Viewing Tasks](#viewing-tasks)
- [Balance & Rewards](#balance--rewards)

> [!TIP]
>
> You can enable debug-level logs with the `-d` option.

### Registration

To serve oracle requests, you **MUST** first register as your desired oracle type, i.e. `generator` or `validator`. These are handled by the registration commands `register` and `unregister` which accepts multiple arguments to register at once. You can then see your registrations with `registrations` command.

Here is an example:

```sh
# 1. Register as both generator and validator
dria-oracle register generator validator

# 2. See that you are registered
dria-oracle registrations

# 3. Unregister from validator
dria-oracle unregister validator
```

> [!NOTE]
>
> You will need to have some tokens in your balance, which will be approved automatically if required by the register command.

### Launching the Node

We launch our node using the `serve` command, followed by models of our choice and the oracle type that we would like to serve.
If we provide no oracle types, it will default to the ones that we are registered to.

```sh
# run as generator
dria-oracle serve -m=gpt-4o-mini -m=llama3.1:latest generator

# run as validator
dria-oracle serve -m=gpt-4o validator

# run as kinds that you are registered to
dria-oracle serve -m=gpt-4o
```

We can start handling tasks from previous blocks until now, and then continue listening for more events:

```sh
# start from 100th block until now, and subscribe to new events
dria-oracle serve -m=gpt-4o --from=100
```

> [!TIP]
>
> You can terminate the application from the terminal as usual (e.g. CTRL+C) to quit the node.

Or, we can handle tasks between specific blocks only, the application will exit upon finishing blocks unlike before:

```sh
# handle tasks between blocks 100 and 500
dria-oracle serve -m=gpt-4o --from=100 --to=500
```

Finally, we can handle an existing task specifically as well (if its unhandled for some reason):

```sh
# note that if task id is given, `from` and `to` will be ignored
dria-oracle serve -m=gpt-4o --task-id <task>
```

> [!WARNING]
>
> Validators must use `gpt-4o` model.

#### Using Arweave

To save from gas fees, an Oracle node can upload its response to Arweave and then store the transaction id of that upload to the contract instead. This is differentiated by looking at the response, and see that it is exactly 64 hexadecimal characters. It is then decoded from hex and encoded to `base64url` format, which can then be used to access the data at `https//arweave.net/{txid-here}`. This **requires** an Arweave wallet.

Following the same logic, the Oracle node can read task inputs from Arweave as well. This **does not require** an Arweave a wallet.

### Viewing Tasks

You can `view` the details of a task by its task id:

```sh
dria-oracle view --task-id <task>
```

You can also view the task status updates between blocks with the same command, by providing `from` and `to` blocks,
which defaults from `earliest` block to `latest` block if none is provided.

```sh
dria-oracle view                      # earliest to latest
dria-oracle view --from=100           # 100      to latest
dria-oracle view --to=100             # earliest to 100
dria-oracle view --from=100 --to=200  # 100      to 200
```

### Balance & Rewards

At any time, you can see your balance with:

```sh
dria-oracle balance
```

As you respond to tasks, you will have rewards available to you. You can see & claim them using your node:

```sh
# print rewards
dria-oracle rewards

# claim all rewards
dria-oracle claim
```

### Making a Request

Although the oracle is only supposed to serve requests made from other parties, it is also able to make requests from the CLI. See usage with the help option:

```sh
dria-oracle request -h
```

It mainly takes an input argument, followed by multiple model arguments:

```sh
dria-oracle request "What is 2+2?" gpt-4o-mini phi3:3.8b
```

## Development

If you would like to contribute, please create an issue first! To start developing, clone the repository:

```sh
git clone https://github.com/firstbatchxyz/dria-oracle-node.git
```

### Testing

Run tests with:

```sh
make test
```

Note that the tests make use of existing contracts, accessed via their contract addresses. The blockchain is forked locally to Anvil and tests are executed over them. Always make sure that you are using the latest contract addresses at [`src/contracts/addresses.rs`](./src/contracts/addresses.rs).
This is especially important if you keep getting "execution reverted" for no reason!

### Documentation

You can view the inline documentation with:

```sh
make docs
```

### Styling

Lint and format with:

```sh
make lint   # clippy
make format # rustfmt
```

## License

This project is licensed under the [Apache License 2.0](https://opensource.org/license/Apache-2.0).
