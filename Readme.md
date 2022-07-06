# Ronin blockchain importer for MongoDB
Imports transactions from ronin into a MongoDB

### Schema:

```json
{
  "sender": String,
  "hash": String,
  "block": Number,
  "created_at": Date
}
```

### Usage:

```shell
cargo build -r
./target/release/rimport3 -h
```
