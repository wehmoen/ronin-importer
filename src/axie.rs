#[macro_use]
extern crate fstrings;

extern crate core;

use hex_literal::hex;
use std::fmt::Debug;
use clap::Parser;
use std::{i64};
use mongodb::{bson::doc, bson::DateTime, sync::Collection, sync::Client, options::FindOneOptions, IndexModel};
use mongodb::options::{IndexOptions, InsertManyOptions};
use serde::{Deserialize, Serialize};
use web3::types::{Address, BlockId, BlockNumber, FilterBuilder, U64};
use web3::ethabi::{Event, EventParam, ParamType, RawLog};
use sha2::{Sha256, Digest};
use sha2::digest::{Update};
use web3::contract::Options;
use tools::database;
use crate::tools::database::MongoDb;

/// Axie Infinity - Axie Transfer importer for MongoDB
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// MongoDB connection URL
    #[clap(long, value_parser, default_value = "mongodb://127.0.0.1:27017")]
    mongodb_uri: String,
    /// MongoDB database name
    #[clap(long, value_parser, default_value = "ronin")]
    mongodb_name: String,
    /// MongoDB collection name
    #[clap(long, value_parser, default_value = "axietransfers")]
    mongodb_collection: String,
    /// Web3 Websocket Host
    #[clap(long, value_parser, default_value = "ws://localhost:8546")]
    web3_hostname: String,
    /// Web3 Provider Type
    #[clap(long, value_parser, default_value = "ws")]
    web3_provider_type: String,
    /// Start Block - Set to 0 to resume from last block in database
    #[clap(long, value_parser, default_value_t = 2678592)]
    start_block: u32,
    /// End Block - If set to anything but 0 the import will stop at this block.
    #[clap(long, value_parser, default_value_t = 0)]
    end_block: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transfer {
    from: String,
    to: String,
    axie: u32,
    block: u32,
    created_at: DateTime,
    transfer_id: String,
}

async fn get_db_head_block(col: &Collection<Transfer>) -> U64 {
    let options = FindOneOptions::builder().sort(doc! {"block": -1}).build();
    let result: Transfer = col.find_one(None, options).unwrap().unwrap();
    return web3::types::U64::from(result.block);
}

fn get_transfer_id(from: &str, to: &str, axie: &u32, block: &u32) -> String {
    let id = f!("{from}{to}{axie}{block}");
    let mut hasher = Sha256::new();
    Update::update(&mut hasher, id.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[tokio::main]
async unsafe fn scan(col: Collection<Transfer>, args: Args) -> web3::Result<()> {
    let transport = match args.web3_provider_type.as_str() {
        "ws" => web3::transports::either::Either::Left(web3::transports::WebSocket::new(&args.web3_hostname).await.unwrap()),
        "http" => web3::transports::either::Either::Right(web3::transports::Http::new(&args.web3_hostname).unwrap()),
        _ => panic!("Invalid provider type")
    };

    let web3 = web3::Web3::new(transport);

    let mut block = if args.start_block == 0 {
        get_db_head_block(&col).await + 1i32
    } else {
        web3::types::U64::from(args.start_block)
    };

    let max_block = if args.end_block == 0 {
        web3.eth().block_number().await.unwrap()
    } else {
        web3::types::U64::from(args.end_block)
    };

    println!("Effective start_block: {}", block);
    println!("Effective end_block: {}", max_block);

    let axie_contract_address: Address = "32950db2a7164ae833121501c797d79e7b79d74c".parse().unwrap();

    let event = Event {
        name: "Transfer".to_string(),
        inputs: vec![
            EventParam {
                name: "_from".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "_to".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "_tokenId".to_string(),
                kind: ParamType::Uint(256),
                indexed: true,
            },
        ],
        anonymous: false,
    };

    loop {
        let filter = FilterBuilder::default()
            .from_block(BlockNumber::from(block))
            .to_block(BlockNumber::from(block))
            .address(vec![axie_contract_address])
            .topics(
                Some(vec![hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").into()]),
                None,
                None,
                None,
            ).build();

        let filter = web3.eth_filter().create_logs_filter(filter).await.unwrap();
        let result: Vec<web3::types::Log> = filter.logs().await.unwrap();
        let completion: f32 = (block.clone().as_u32() as f32 / max_block.clone().as_u32() as f32) * 100f32;
        if result.len() > 0 {
            let mut tx_pool: Vec<Transfer> = vec![];
            for log in result {
                let rl = RawLog {
                    topics: log.topics,
                    data: log.data.clone().0,
                };

                let data = event.parse_log(rl);
                let params = data.unwrap().params;

                let from = &params[0].value.to_string();
                let from = f!("0x{from}");
                let to = &params[1].value.to_string();
                let to = f!("0x{to}");
                let token = params[2].clone().value.into_uint().unwrap().as_u32();

                let block = web3.eth().block(BlockId::from(log.block_hash.unwrap())).await.unwrap().unwrap();

                let timestamp = block.timestamp.as_u64() * 1000;
                let timestamp = DateTime::from_millis(i64::try_from(timestamp).unwrap());

                let block = block.number.unwrap().as_u32();
                let transfer_id = get_transfer_id(&from, &to, &token, &block);
                let tx: Transfer = Transfer {
                    from,
                    to,
                    axie: token,
                    block,
                    created_at: timestamp,
                    transfer_id: transfer_id.to_owned(),
                };
                tx_pool.push(tx);
            }

            println!("Importing {} transfers in block {} ({:.6}%)", tx_pool.len(), block, completion);
            if tx_pool.len() > 0 {
                let insert_options = InsertManyOptions::builder().ordered(false).build();
                col.insert_many(tx_pool, insert_options).ok();
            }
        } else {
            println!("Importing 0 transfers in block {} ({:.6}%)", block, completion);
        }

        block = block + 1i32;

        if block > max_block {
            println!("Breaking!");
            break;
        }
    }

    Ok(())
}

mod tools;

#[tokio::main]
async fn main() -> Result<(), ()> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const NAME: &str = env!("CARGO_PKG_NAME");

    println!("{} Axie Transfer Importer v{}", NAME, VERSION);

    let args: Args = Args::parse();

    let db = MongoDb::new(database::Options { client_uri: String::from(&args.mongodb_uri), database: String::from(&args.mongodb_name) }).await;
    let collection = db.database.collection::<Transfer>(&args.mongodb_collection);

    let options = IndexOptions::builder().unique(true).build();
    let index_model = IndexModel::builder().keys(doc! {"transfer_id": 1u32}).options(options).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"axie": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"from": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"to": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"block": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let scan_result = tokio::task::spawn_blocking(|| {
        unsafe { scan(collection, args) }
    }).await.expect("Scan process panicked. We provided some meds but had to exit anyways.");

    let result = match scan_result {
        Ok(_res) => std::string::String::from("Finished importing axie transfers!"),
        Err(_error) => format!("{} {}", &"An error occured during the process of importing axie transfer!", _error)
    };

    println!("{}", result);

    db.update_health("axie-transfers".into());

    Ok(())
}