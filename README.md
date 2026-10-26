# Ditto _(ditto)_

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

Asynchronous Rust message queue and event routing broker service with Kafka and gRPC integration.

Ditto is a backend event routing and queue persistence broker built in Rust using Tokio, SQLx (PostgreSQL), rdkafka, and Tonic gRPC for high-throughput distributed message processing.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
- [Maintainers](#maintainers)
- [Contributing](#contributing)
- [License](#license)

## Background

Distributed architectures require robust, persistent queue management and event streaming capable of tracking message states, queue servers, and step transitions. Ditto manages queue topologies backed by PostgreSQL and Apache Kafka event streams.

## Install

Requires Rust 1.75+ and Cargo:

```bash
git clone https://github.com/Nevitash/ditto.git
cd ditto
cargo build
```

## Usage

Run database migrations and launch the broker daemon:

```bash
cargo run
```

Run test suite:

```bash
cargo test
```

## Maintainers

[@Nevitash](https://github.com/Nevitash)

## Contributing

PRs accepted. Please open an issue first to discuss architectural changes or protocol buffer updates.

## License

[MIT](LICENSE)
