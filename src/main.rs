use std::fmt::Debug;
use clap::Parser;
use std::i64;
use mongodb::{
    bson::doc,
    sync::Client,
};
use mongodb::bson::DateTime;
use mongodb::sync::Collection;
use serde::{Deserialize, Serialize};
use web3::types::{BlockId, BlockNumber};

/// Ronin blockchain importer for MongoDB
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// MongoDB Connection URL
    #[clap(long, value_parser, default_value = "mongodb://127.0.0.1:27017")]
    mongodb_uri: String,
    /// MongoDB database
    #[clap(long, value_parser, default_value = "roninrest")]
    mongodb_name: String,
    /// Web3 Websocket Host
    #[clap(long, value_parser, default_value = "ws://localhost:8546")]
    web3_hostname: String,
    /// Start Block
    #[clap(long, value_parser, default_value_t = 1)]
    start_block: u32


}

#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    sender: String,
    hash: String,
    block: u32,
    created_at: DateTime,
}

#[tokio::main]
async fn scan(col: Collection<Transaction>, args: Args) -> web3::Result<()> {
    let transport = web3::transports::WebSocket::new(&args.web3_hostname).await.unwrap();
    let web3 = web3::Web3::new(transport);

    let mut block= web3::types::U64::from(args.start_block);
    let max_block = web3.eth().block_number().await.unwrap();
    loop {
        let block_data = web3.eth().block_with_txs(BlockId::Number(BlockNumber::from(block))).await.unwrap().unwrap();
        let txs = block_data.transactions;
        if txs.len() > 0 {
            let ts = block_data.timestamp.as_u64() * 1000;
            let mut tx_pool = vec![];
            println!("Block: {}\tTransactions: {}", block, txs.len());
            for tx in txs {
                tx_pool.push(Transaction {
                    sender: str::replace(&web3::helpers::to_string(&tx.from), "\"", ""),
                    hash: str::replace(&web3::helpers::to_string(&tx.hash), "\"",""),
                    block: block_data.number.unwrap().as_u32(),
                    created_at: DateTime::from_millis(i64::try_from(ts).unwrap()),
                });
            }
            col.insert_many(tx_pool, None).unwrap();
        } else{
            println!("Block: {}\tTransactions: {}", block, 0);
        }
        block = block + 1;
        if block > max_block {
            println!("Breaking!");
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {

    let args: Args = Args::parse();

    let client = Client::with_uri_str(&args.mongodb_uri).unwrap();
    let database = client.database(&args.mongodb_name);
    let collection = database.collection::<Transaction>("transactions");

    tokio::task::spawn_blocking(|| {
        scan(collection, args)
    }).await.expect("Task panicked");

    Ok(())
}
