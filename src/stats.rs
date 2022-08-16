use mongodb::bson::{doc};
use mongodb::options::{FindOneOptions, InsertManyOptions};
use mongodb::sync::Collection;
use serde::{Deserialize, Serialize};
use web3;
use web3::transports::WebSocket;
use web3::types::{BlockId, BlockNumber};

use tools::database::{MongoDb, Options};

mod tools;

#[derive(Debug, Serialize, Deserialize)]
struct BlockStats {
    block: isize,
    tx_num: isize,
}

async fn get_db_head_block(col: &Collection<BlockStats>) -> isize {
    let options = FindOneOptions::builder().sort(doc! {"block": -1i64}).build();
    let result = col.find_one(None, options).unwrap();
    match result {
        None => 0,
        Some(stats) => stats.block + 1
    }
}

const WEB3_PROVIDER: &str = "ws://localhost:8546";
const MONGODB_URI: &str = "mongodb://127.0.0.1:27017";
const MONGODB_NAME: &str = "roninstatistics";

const MONGODB_BLOCK_TABLE: &str = "blocks";

#[tokio::main]
async fn main() {
    let web3 = web3::Web3::new(WebSocket::new(WEB3_PROVIDER).await.unwrap());
    let db = MongoDb::new(Options { client_uri: String::from(MONGODB_URI), database: String::from(MONGODB_NAME) }).await;
    let block_stats = db.database.collection::<BlockStats>(MONGODB_BLOCK_TABLE);

    let mut block = get_db_head_block(&block_stats).await;
    let chain_head: isize = (web3.eth().block_number().await.unwrap().as_u64() - 50u64) as isize;

    let mut stats: Vec<BlockStats> = vec![];

    loop {
        let block_tx_num = match web3.eth().block_transaction_count(
            BlockId::from(
                BlockNumber::from(block as u64)
            )
        ).await.unwrap() {
            None => 0,
            Some(stats) => stats.as_u32()
        };

        stats.push(
            BlockStats {
                block: block.to_owned(),
                tx_num: block_tx_num as isize,
            }
        );

        block = block + 1;

        if stats.len() >= 10000 || block >= chain_head {
            block_stats.insert_many(
                &stats,
                InsertManyOptions::builder().ordered(false).build(),
            ).ok();
            println!("Processed blocks {} to {}", &stats[0].block, &stats[stats.len()-1].block);
            stats.clear();
        }

        if block >= chain_head {
            break;
        }
    }
}