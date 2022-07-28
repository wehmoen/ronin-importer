#[macro_use]
extern crate fstrings;
use clap::Parser;
use hex_literal::hex;
use mongodb::bson::{DateTime, doc};
use mongodb::IndexModel;
use mongodb::options::{FindOneOptions, IndexOptions, InsertManyOptions};
use mongodb::sync::{Client, Collection};
use sha2::{Sha256, Digest};
use sha2::digest::{Update};
use web3::api::BaseFilter;
use web3::ethabi::{Address, Event, RawLog};
use web3::transports::{Either, Http, WebSocket};
use web3::types::{BlockNumber, FilterBuilder, Log};

use crate::contracts::contracts::{Contract, ContractType};
use crate::contracts::database::Transfer;
use crate::tools::database::{MongoDb, Options};

mod contracts;

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
    #[clap(long, value_parser, default_value = "tokentransfers")]
    mongodb_collection: String,
    /// Web3 Websocket Host
    #[clap(long, value_parser, default_value = "ws://localhost:8546")]
    web3_hostname: String,
    /// Web3 Provider Type
    #[clap(long, value_parser, default_value = "ws")]
    web3_provider_type: String,
    /// Start Block - Set to 0 to resume from last block in database
    #[clap(long, value_parser, default_value_t = 1)]
    start_block: u32,
    /// End Block - If set to anything but 0 the import will stop at this block.
    #[clap(long, value_parser, default_value_t = 0)]
    end_block: u32,
}

async fn get_db_head_block(col: &Collection<Transfer>) -> web3::types::U64 {
    let options: FindOneOptions = FindOneOptions::builder().sort(doc! {"block": -1i64}).build();
    let result = col.find_one(None, options).unwrap().unwrap_or(Transfer::empty());
    return web3::types::U64::from(result.block);
}


fn get_transfer_id(hash: String, index: String) -> String {
    let id = f!("{hash}-{index}");
    let mut hasher = Sha256::new();
    Update::update(&mut hasher, id.as_bytes());
    format!("{:x}", hasher.finalize())
}

mod tools;

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let erc_20_transfer: Event = contracts::events::erc_20_transfer();
    let erc_721_transfer: Event = contracts::events::erc_721_transfer();
    let contracts: contracts::contracts::ContractList = contracts::contracts::default();

    let transport = match args.web3_provider_type.as_str() {
        "ws" => Either::Left(WebSocket::new(&args.web3_hostname).await.unwrap()),
        "http" => Either::Right(Http::new(&args.web3_hostname).unwrap()),
        _ => panic!("Invalid provider type")
    };
    let web3 = web3::Web3::new(transport);

    let db = MongoDb::new(Options { client_uri: String::from(&args.mongodb_uri), database: String::from(&args.mongodb_name) }).await;
    let collection = db.database.collection::<Transfer>(&args.mongodb_collection);

    collection.create_index(IndexModel::builder().keys(doc! {"log_id": 1u32}).options(IndexOptions::builder().unique(true).build()).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"from": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"to": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"token": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"value_or_token_id": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"block": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"erc": 1u32}).build(), None).expect("Failed to create index!");
    collection.create_index(IndexModel::builder().keys(doc! {"transaction_id": 1u32}).build(), None).expect("Failed to create index!");

    let mut block = if args.start_block == 0 {
        get_db_head_block(&collection).await + 1i32
    } else {
        web3::types::U64::from(args.start_block)
    };

    let max_block =
        if args.end_block == 0 {
            web3.eth().block_number().await.unwrap()
        } else {
            web3::types::U64::from(args.end_block)
        };

    loop {
        let mut num_erc_20_transfers = 0;
        let mut num_erc_721_transfers = 0;
        let mut tx_pool: Vec<Transfer> = vec![];

        for address in contracts.keys() {
            let contract: &Contract = contracts.get(address).unwrap();
            let address: Address = address.parse().unwrap();

            let transfer_filter = FilterBuilder::default()
                .from_block(BlockNumber::from(block))
                .to_block(BlockNumber::from(block))
                .address(vec![address])
                .topics(
                    Some(vec![hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").into()]),
                    None,
                    None,
                    None,
                ).build();

            let filter: BaseFilter<Either<WebSocket, Http>, Log> = web3.eth_filter().create_logs_filter(transfer_filter).await.unwrap();
            let result: Vec<Log> = filter.logs().await.unwrap();

            if result.len() > 0 {

                for log in result {

                    let raw_log = RawLog {
                        topics: log.topics,
                        data: log.data.0
                    };

                    let transfer: Transfer = match contract.erc {
                        ContractType::ERC20 => {
                            num_erc_20_transfers+=1;
                            let data = erc_20_transfer.clone().parse_log(raw_log);
                            let data = data.unwrap().params;

                            Transfer {
                                from: data[0].value.to_string(),
                                to: data[1].value.to_string(),
                                token: web3::helpers::to_string(&address).replace("\"", ""),
                                value_or_token_id: data[2].value.to_string(),
                                created_at: DateTime::from_millis(chrono::Utc::now().timestamp() * 1000),
                                block: block.clone().as_u64(),
                                transaction_id: web3::helpers::to_string(&log.transaction_hash.unwrap()).replace("\"", ""),
                                erc: ContractType::ERC20,
                                log_index: web3::helpers::to_string(&log.log_index.unwrap()).replace("\"", ""),
                                log_id: get_transfer_id(web3::helpers::to_string(&log.transaction_hash.unwrap()), web3::helpers::to_string(&log.log_index.unwrap()))
                            }
                        }
                        ContractType::ERC721 => {
                            num_erc_721_transfers+=1;
                            let data = erc_721_transfer.clone().parse_log(raw_log);
                            let data = data.unwrap().params;

                            Transfer {
                                from: data[0].value.to_string(),
                                to: data[1].value.to_string(),
                                token: web3::helpers::to_string(&address).replace("\"", ""),
                                value_or_token_id: data[2].value.to_string(),
                                created_at: DateTime::from_millis(chrono::Utc::now().timestamp() * 1000),
                                block: block.clone().as_u64(),
                                transaction_id: web3::helpers::to_string(&log.transaction_hash.unwrap()).replace("\"", ""),
                                erc: ContractType::ERC721,
                                log_index: web3::helpers::to_string(&log.log_index.unwrap()).replace("\"", ""),
                                log_id: get_transfer_id(web3::helpers::to_string(&log.transaction_hash.unwrap()), web3::helpers::to_string(&log.log_index.unwrap()))
                            }
                        }
                        ContractType::Unknown => continue
                    };

                    tx_pool.push(transfer);
                }
            }
        }

        if tx_pool.len() > 0 {
            let insert_options = InsertManyOptions::builder().ordered(false).build();
            collection.insert_many(tx_pool, insert_options).ok();
        }

        println!("Block: {}\t\tERC20 Transfers: {}\tERC721 Transfers: {}", block, num_erc_20_transfers, num_erc_721_transfers);

        block = block + 1i32;

        if block > max_block {
            println!("Breaking!");
            break;
        }
    }
    db.update_health(String::from("erc-transfer"));
}