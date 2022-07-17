# Ronin blockchain importer for MongoDB
Imports transactions or axie transfer from ronin into a MongoDB

### Transaction Schema:

```json
{
  "from": String,
  "to": String,
  "hash": String,
  "block": Number,
  "created_at": Date
}
```

### Axie Transfer Schema

```json
{
  "from": String,
  "to": String,
  "axie": Number,
  "block": Number,
  "created_at": String,
  "transfer_id": String:sha256(from, to, axie, block)
}
```

### Usage:

```shell
cargo build -r
./target/release/transactions -h // Transaction importer
./target/release/axie-transfer -h // Axie transfer history importer
```
