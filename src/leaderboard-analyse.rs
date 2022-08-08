use std::ops::Deref;
use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::sync::{Collection, Cursor};
use web3::transports::WebSocket;

use tools::database::{MongoDb, Options};

use crate::tools::types::{BattleId, ClientId, PVPBattleLog};

mod tools;

#[tokio::main]
async fn main() {
    let db_options = Options {
        client_uri: "mongodb://127.0.0.1".to_string(),
        database: "ronin".to_string(),
    };

    let ws = WebSocket::new("ws://localhost:8546").await.unwrap();
    let provider = web3::Web3::new(ws);



    // let gene_str = "0x11c642400a028ca14a428c20cc011080c61180a0820180604233082";
    // let decoder = agp::agp::AxieGeneDecoder::new(gene_str, None);
    // let decoded = decoder.parse();

    // println!("{:?}", decoded);
    //
    // struct Statistic {
    //     total_battlelogs: i32,
    // }
    //
    // let db = MongoDb::new(db_options).await;
    // let battlelogs: Collection<PVPBattleLog> = db.database.collection::<PVPBattleLog>("pvpbattlelogs");
    // let mut logs_to_analyse = battlelogs.aggregate(vec![
    //     doc! { "$sample": { "size": 5i64 } }
    // ], None).unwrap();
    //
    // println!("hi");
    //
    // while let Some(log) = logs_to_analyse.next() {
    //     let battle_id = log.unwrap();
    //     println!("{:?}", battle_id.first_client_fighters);
    // }
    println!("done")
}