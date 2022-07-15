#[macro_use]
extern crate fstrings;

extern crate core;

use hex_literal::hex;
use std::fmt::Debug;
use clap::Parser;
use std::{i64, thread};
use std::time::Duration;
use mongodb::{
    bson::doc,
    bson::DateTime,
    sync::Collection,
    sync::Client,
    options::FindOneOptions,
};
use serde::{Deserialize, Serialize};
use web3::types::{Address, BlockId, BlockNumber, FilterBuilder, U64};
use web3::contract::{Contract};
use web3::ethabi::{Event, EventParam, ParamType, RawLog};

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
    /// Start Block - Set to 0 to resume from last block in database
    #[clap(long, value_parser, default_value_t = 2678592)]
    start_block: u32,
    /// End Block - If set to anything but 0 the import will stop at this block.
    #[clap(long, value_parser, default_value_t = 0)]
    end_block: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Transfer {
    from: String,
    to: String,
    axie: u32,
    block: u32,
    created_at: DateTime,
}

async fn get_db_head_block(col: &Collection<Transfer>) -> web3::types::U64 {
    let options = FindOneOptions::builder().sort(doc! {"block": -1}).build();
    let result: Transfer = col.find_one(None, options).unwrap().unwrap();
    return web3::types::U64::from(result.block);
}

#[tokio::main]
async fn scan(col: Collection<Transfer>, args: Args) -> web3::Result<()> {
    let transport = web3::transports::WebSocket::new(&args.web3_hostname).await.unwrap();
    let web3 = web3::Web3::new(transport);

    let mut block: U64 = if args.start_block == 0 {
        get_db_head_block(&col).await + web3::types::U64::from("1")
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
    println!("Starting in 5 seconds...");

    let abi = include_str!("abi.json").as_bytes();
    let axie_contract_address: Address = "32950db2a7164ae833121501c797d79e7b79d74c".parse().unwrap();
    let contract = Contract::from_json(web3.eth(), axie_contract_address, abi).unwrap();

    let params: Vec<EventParam> = vec![
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
    ];

    let event = Event {
        name: "Transfer".to_string(),
        inputs: params,
        anonymous: false,
    };

    thread::sleep(Duration::from_secs(5));
    loop {
        let max = block + web3::types::U64::from("10");

        let filter = FilterBuilder::default()
            .from_block(BlockNumber::from(block))
            .to_block(BlockNumber::from(max))
            .address(vec![contract.address()])
            .topics(
                Some(vec![hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").into()]),
                None,
                None,
                None,
            ).build();

        let filter = web3.eth_filter().create_logs_filter(filter).await.unwrap();
        let result: Vec<web3::types::Log> = filter.logs().await.unwrap();
        println!("Importing {} transfers in block range from {} to {}", result.len(), block, max);

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

                tx_pool.push(Transfer {
                    from,
                    to,
                    axie: token,
                    block: block.number.unwrap().as_u32(),
                    created_at: timestamp,
                });

            }

            if tx_pool.len() > 0 {
                    col.insert_many(tx_pool, None).unwrap();
            }
        }

        block = block + web3::types::U64::from("10");

        if block > max_block  {
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

    println!("{} Axie Transfer Importer v{}", NAME, VERSION);

    let args: Args = Args::parse();

    let client = Client::with_uri_str(&args.mongodb_uri).unwrap();
    let database = client.database(&args.mongodb_name);
    let collection = database.collection::<Transfer>(&args.mongodb_collection);

    let scan_result = tokio::task::spawn_blocking(|| {
        scan(collection, args)
    }).await.expect("Scan process panicked. We provided some meds but had to exit anyways.");

    let result = match scan_result {
        Ok(_res) => std::string::String::from("Finished importing axie transfers!"),
        Err(_error) => format!("{} {}", &"An error occured during the process of importing axie transfer!", _error)
    };

    println!("{}", result);

    Ok(())
}
