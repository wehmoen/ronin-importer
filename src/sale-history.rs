extern crate core;
#[macro_use]
extern crate fstrings;

use std::i64;
use std::fmt::Debug;

use clap::Parser;
use hex_literal::hex;
use mongodb::{bson::DateTime, bson::doc, IndexModel, options::FindOneOptions, sync::Client, sync::Collection};
use mongodb::options::{IndexOptions, InsertManyOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha2::digest::Update;
use web3::ethabi::{Event, EventParam, ParamType, RawLog, Token, Uint};
use web3::types::{Address, BlockId, BlockNumber, FilterBuilder, U64};

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
    #[clap(long, value_parser, default_value = "axiesales")]
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
struct Sale {
    seller: String,
    buyer: String,
    axie: usize,
    block: u32,
    price: String,
    token: String,
    transaction_id: String,
    created_at: DateTime
}



async fn get_db_head_block(col: &Collection<Sale>) -> U64 {
    let options = FindOneOptions::builder().sort(doc! {"block": -1}).build();
    let result: Sale = col.find_one(None, options).unwrap().unwrap();
    return web3::types::U64::from(result.block);
}

#[tokio::main]
async unsafe fn scan(col: Collection<Sale>, args: Args) -> web3::Result<()> {
    let transport = match args.web3_provider_type.as_str() {
        "ws" => web3::transports::either::Either::Left(web3::transports::WebSocket::new(&args.web3_hostname).await.unwrap()),
        "http" => web3::transports::either::Either::Right(web3::transports::Http::new(&args.web3_hostname).unwrap()),
        _ => panic!("Invalid provider type")
    };

    let web3 = web3::Web3::new(transport);

    let mut block = if args.start_block == 0 {
        web3::types::U64::from(get_db_head_block(&col).await + 1i32)
    } else {
        web3::types::U64::from(args.start_block)
    };

    let max_block = if args.end_block == 0 {
        web3.eth().block_number().await.unwrap()
    } else {
        web3::types::U64::from(args.end_block)
    };

    let range = (max_block.clone().as_usize() as f32 - block.clone().as_usize() as f32) as f32;

    println!("Effective start_block: {}", block);
    println!("Effective end_block: {}", max_block);

    let contract_address: Address = "213073989821f738A7BA3520C3D31a1F9aD31bBd".parse().unwrap();
    let axie_contract_address: Address = "32950db2a7164ae833121501c797d79e7b79d74c".parse().unwrap();

    let axie_transfer_event = Event {
        name: "Transfer".to_string(),
        inputs: vec![
            EventParam {
                name: "_from".to_string(),
                kind: ParamType::Address,
                indexed: true
            },
            EventParam {
                name: "to".to_string(),
                kind: ParamType::Address,
                indexed: true
            },
            EventParam {
                name: "_tokenId".to_string(),
                kind: ParamType::Uint(256),
                indexed: true
            },

        ],
        anonymous: false
    };

    let auction_successful_event = Event {
        name: "AuctionSuccessful".to_string(),
        inputs: vec![
            EventParam {
                name: "_seller".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "_buyer".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "_listingIndex".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "_token".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "_totalPrice".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    };

    loop {
        let filter = FilterBuilder::default()
            .from_block(BlockNumber::from(block))
            .to_block(BlockNumber::from(block))
            .address(vec![contract_address])
            .topics(
                Some(vec![hex!("0c0258cd7f0d9474f62106c6981c027ea54bee0b323ea1991f4caa7e288a5725").into()]),
                None,
                None,
                None,
            ).build();

        let filter = web3.eth_filter().create_logs_filter(filter).await.unwrap();
        let result: Vec<web3::types::Log> = filter.logs().await.unwrap();
        let completion: f32 = (block.clone().as_u32() as f32 / range) * 100f32;
        if result.len() > 0 {
            let mut tx_pool: Vec<Sale> = vec![];
            for log in result {

                let rl = RawLog {
                    topics: log.topics,
                    data: log.data.clone().0,
                };

                let data = auction_successful_event.parse_log(rl);
                let params = data.unwrap().params;

                let seller = &params[0].value.to_string();
                let seller = f!("0x{seller}");
                let buyer = &params[1].value.to_string();
                let buyer = f!("0x{buyer}");

                let listingIndex = params[2].clone().value.into_uint().unwrap().as_usize();
                let token = &params[3].value.to_string();
                let token = f!("0x{token}");
                let totalPrice = params[4].clone().value.into_uint().unwrap();

                let block = web3.eth().block(BlockId::from(log.block_hash.unwrap())).await.unwrap().unwrap();

                let timestamp = block.timestamp.as_u64() * 1000;
                let timestamp = DateTime::from_millis(i64::try_from(timestamp).unwrap());

                let block = block.number.unwrap().as_u32();

                let tx_hash = log.transaction_hash.unwrap();
                let mut other_logs = web3.eth().transaction_receipt(tx_hash.clone()).await.unwrap().unwrap();

                for o_log in other_logs.logs {
                    if o_log.address == axie_contract_address {

                        let axie_transfer_raw = RawLog {
                            topics: o_log.topics,
                            data: o_log.data.clone().0,
                        };

                        let axie_transfer_data = axie_transfer_event.parse_log(axie_transfer_raw).unwrap();
                        let axie: Token = axie_transfer_data.params[2].clone().value;
                        let axie: usize = axie.into_uint().unwrap().as_usize();

                        let tx: Sale = Sale {
                            seller: seller.to_owned(),
                            buyer: buyer.to_owned(),
                            axie,
                            block,
                            price: totalPrice.to_string(),
                            token: token.to_owned(),
                            transaction_id: web3::helpers::to_string(&tx_hash).replace("\"", ""),
                            created_at: timestamp
                        };
                        tx_pool.push(tx);
                    }
                }


            }

            println!("Importing {} sales in block {} ({:.6}%)", tx_pool.len(), block, completion);
            if tx_pool.len() > 0 {
                let insert_options = InsertManyOptions::builder().ordered(false).build();
                col.insert_many(tx_pool, insert_options).ok();
            }
        } else {
            println!("Importing 0 sales in block {} ({:.6}%)", block, completion);
        }

        block = block + 1i32;

        if block > max_block {
            println!("Breaking!");
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const NAME: &str = env!("CARGO_PKG_NAME");

    println!("{} Axie Sale Importer v{}", NAME, VERSION);

    let args: Args = Args::parse();

    let client = Client::with_uri_str(&args.mongodb_uri).unwrap();
    let database = client.database(&args.mongodb_name);
    let collection = database.collection::<Sale>(&args.mongodb_collection);

    let options = IndexOptions::builder().unique(true).build();
    let index_model = IndexModel::builder().keys(doc! {"transaction_id": 1u32}).options(options).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"buyer": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"seller": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"axie": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"block": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let index_model = IndexModel::builder().keys(doc! {"created_at": 1u32}).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let _ = tokio::task::spawn_blocking(|| {
        unsafe { scan(collection, args) }
    }).await.expect("Scan process panicked. We provided some meds but had to exit anyways.");

    // let result = match scan_result {
    //     Ok(_res) => std::string::String::from("Finished importing axie transfers!"),
    //     Err(_error) => format!("{} {}", &"An error occured during the process of importing axie transfer!", _error)
    // };

    // println!("{}", result);

    Ok(())
}
