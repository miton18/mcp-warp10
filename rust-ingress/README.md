# rust-ingress

Experimental Warp 10 ingress written in Rust, with a configurable policy on the
`.owner` / `.producer` / `.app` internal labels (the stock ingress always forces
them from the write token and rejects tokens without them with HTTP 403).

This is an experiment scaffold. The goal is parity with the stock distributed
`Ingress` (`senx/warp10-platform`, `io.warp10.continuum.ingress.Ingress`) on the
Kafka contract, plus a relaxed label policy.

## What it does

`POST /api/v0/update` with header `X-Warp10-Token`, body in GTS input format
(`<ts>[/lat:lon[/elev]] class{labels} value` per line), exactly like the stock
endpoint. For each line it:

1. parses the GTS (`gts.rs`),
2. resolves the final labels (`policy.rs`),
3. computes `classId` / `labelsId` with the platform SipHash keys (`ids.rs`,
   `siphash.rs`),
4. encodes the datapoint in the `GTSEncoder` binary format (`encoder.rs`),
5. publishes a thrift `KafkaDataMessage` (type `STORE`) on the data topic with
   key = 16 bytes `classId || labelsId` big-endian, and a thrift `Metadata`
   (key-prefixed) on the meta topic for each new GTS (`thrift.rs`, `kafka.rs`,
   `server.rs`).

## Label policy (the point of the experiment)

Stock behavior (`StandaloneIngressHandler.java`): `.producer` / `.owner` /
`.app` are always taken from the write token and overwrite client labels; a
token without owner/producer is rejected with 403 before any parsing.

Here (`policy.rs`):

- if the token defines `owner` / `producer` / `app`, they are forced (stock
  behavior);
- otherwise, if the token has the attribute `.allowowner` / `.allowproducer`,
  the client-pushed `.owner` / `.producer` label is kept;
- otherwise the behavior is configurable: reject (stock-like, default) or fall
  back to a default UUID.

Token labels (the `labels` map of a write token) still cannot set
`.owner` / `.producer` / `.app`, matching the stock rule.

## Configuration (env)

| Var | Default | Meaning |
| --- | --- | --- |
| `RUST_INGRESS_LISTEN` | `0.0.0.0:8082` | HTTP listen address |
| `RUST_INGRESS_KAFKA_BROKERS` | `localhost:9092` | Kafka bootstrap (used with `RUST_INGRESS_PRODUCER=kafka`) |
| `RUST_INGRESS_DATA_TOPIC` | `data` | data topic (`ingress.kafka.data.topic`) |
| `RUST_INGRESS_META_TOPIC` | `metadata` | metadata topic (`ingress.kafka.meta.topic`) |
| `RUST_INGRESS_CLASS_KEY_HEX` | — | 16-byte hex SipHash key for classIds (platform `SIPHASH_CLASS` key) |
| `RUST_INGRESS_LABELS_KEY_HEX` | — | 16-byte hex SipHash key for labelsIds (platform `SIPHASH_LABELS` key) |
| `RUST_INGRESS_TOKENS_FILE` | — | JSON map `token string -> {owner, producer, app, labels, attributes}` |
| `RUST_INGRESS_MISSING_OWNER` / `..._PRODUCER` | `reject` | `reject` or `default` |
| `RUST_INGRESS_DEFAULT_OWNER` / `..._PRODUCER` | zero UUID | fallback UUID when `default` |
| `RUST_INGRESS_ALLOW_OWNER_ATTR` / `..._PRODUCER_ATTR` | `.allowowner` / `.allowproducer` | token attribute allowing client-pushed internal labels |
| `RUST_INGRESS_PRODUCER` | `mock` | `mock` (records messages in memory) or `kafka` (needs `--features kafka`) |

Example tokens file:

```json
{
  "platform-write-token": {
    "producer": "22222222-2222-2222-2222-222222222222",
    "app": "metrics",
    "attributes": [".allowowner"]
  }
}
```

With this token, a payload line like `1// m{.owner=<tenant-uuid>} 1` is stored
with `.owner` = the tenant UUID, without minting one Warp 10 write token per
tenant.

## Build / test / run

```bash
cargo test                 # unit tests (siphash vectors, parser, policy, thrift, encoder, pipeline)
cargo run                  # mock producer (messages kept in memory), port 8082
cargo run --features kafka # real Kafka producer via rdkafka
```

## Kafka contract (must match the stock ingress byte-for-byte)

- data topic: key = 16 bytes `classId || labelsId` (big-endian, `GTSHelper.fillGTSIds`),
  value = thrift binary `KafkaDataMessage` (`type=STORE`, `classId`, `labelsId`,
  `data` = encoder bytes, optional `metadata`, optional `attributes` such as
  `fdb.tenant.prefix`) — see `Ingress.java` `sendDataMessage`;
- meta topic: key = same 16 bytes, value = key prefix + thrift binary `Metadata`
  (`name`, `labels` incl. `.owner`/`.producer`/`.app`, `classId`, `labelsId`,
  `attributes`) — see `Ingress.java` `pushMetadataMessage`;
- `classId` = palindromic SipHash-2-4 of the UTF-8 class name;
  `labelsId` = two-phase palindromic SipHash over per-label name/value hashes
  sorted by name hash, then value hash, each hash serialized big-endian
  (`GTSHelper.labelsId`);
- deletes would be `KafkaDataMessage` with `type=DELETE` (not implemented yet).

## Known limitations / parity work

- Real Warp 10 tokens are AES-wrapped thrift and are not decrypted here; the
  experiment maps token strings to claims via `RUST_INGRESS_TOKENS_FILE`.
  Production would decrypt tokens or plug in an `AuthenticationPlugin`.
- Kafka message AES wrap / SipHash MAC (`aesDataKey`, `siphashDataKey`,
  `AES_KAFKA_META`) not implemented yet.
- Encoder v1 supports long / double / boolean / string / bytes values, location
  and elevation; BigDecimal encoding and encoder AES wrap are not ported.
- `GeoXPLib.toGeoXPPoint` packing is implemented as `(latE7 << 32) | lonE7` —
  validate against the platform.
- Parity gate before any real use: produce identical payloads through the stock
  ingress and this one and diff the Kafka messages (ids, encoder bytes, thrift),
  and validate `classId`/`labelsId` against `GTSHelper` on the Java side.
- `/api/v0/meta` and `/api/v0/delete` endpoints are not implemented.
