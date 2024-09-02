<p align="center">
  <img src="https://raw.githubusercontent.com/firstbatchxyz/dria-js-client/master/logo.svg" alt="logo" width="142">
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
cargo install --git https://github.com/firstbatchxyz/dkn-l2-oracle
```

This will create a binary called `dkn-oracle`. You can see the available commands with:

```sh
dkn-oracle help
```

## Setup

TODO: !!!

## Usage

The CLI provides several methods to interact with the oracle contracts.

- [Registration](#registration)
- [Launching the Node](#launching-the-node)
- [Viewing Tasks](#viewing-tasks)
- [Balance & Rewards](#balance--rewards)

### Registration

To serve oracle requests, you must first register as your desired oracle type, i.e. `generator` or `validator`. These are handled by the registration commands `register` and `unregister` which accepts multiple arguments to register at once. You can then see your registrations with `registrations` command.

Here is an example:

```sh
# 1. Register as both generator and validator
dkn-oracle register generator validator

# 2. See that you are registered
dkn-oracle registrations

# 3. Unregister from validator
dkn-oracle unregister validator
```

> [!NOTE]
>
> You will need to have some tokens in your balance, which will be approved automatically if required by the register command.

### Launching the Node

We launch our node using the `start` command, followed by models of our choice and the oracle type that we would like to serve.
If we provide no oracle types, it will default to the ones that we are registered to.

```sh
dkn-oracle start -m=gpt-4o-mini -m=llama3.1:latest
```

You can terminate the application from the terminal as usual (e.g. Control+C) to quit the node.

### Viewing Tasks

You can view the status of a task by its task id:

```sh
dkn-oracle view <task-id>
```

You can also view the task status updates between blocks with the `tasks` command.
It accepts `--from` and `--to` arguments to indicate block numbers or tags, defaults from `earliest` block to `latest` block.

```sh
dkn-oracle tasks                      # earliest to latest
dkn-oracle tasks --from=100           # 100      to latest
dkn-oracle tasks --to=100             # earliest to 100
dkn-oracle tasks --from=100 --to=200  # 100      to 200
```

### Balance & Rewards

At any time, you can see your balance with:

```sh
dkn-oracle balance
```

As you respond to tasks, you will have rewards available to you. You can see & claim them using your node:

```sh
# print rewards
dkn-oracle rewards

# claim all rewards
dkn-oracle claim
```

### Making a Request

Although the oracle is only supposed to serve requests made from other parties, it is also able to make requests from the CLI. See usage with the help option:

```sh
dkn-oracle request -h
```

It mainly takes an input argument, followed by multiple model arguments:

```sh
dkn-oracle request "What is 2+2?" gpt-4o-mini phi3:3.8b
```

> [!NOTE]
>
> Making a request from the Oracle node is mainly for testing purposes, and you are not expected to use this command at all. Furthermore, it is only used to make plaintext requests, instead of larger ones via Arweave or more complex ones via Workflows.

## Development

If you would like to contribute, please create an issue first! To start developing, clone the repository:

```sh
git clone https://github.com/firstbatchxyz/dkn-compute-node.git
```

### Testing

Run tests with:

```sh
make test
```

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
