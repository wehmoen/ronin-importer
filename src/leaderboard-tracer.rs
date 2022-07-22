#![allow(non_snake_case)]
#[macro_use]
extern crate fstrings;

use crate::tools::types::*;
use mongodb::{bson::doc, sync::Collection, sync::Client, IndexModel};
use mongodb::options::{IndexOptions, InsertManyOptions};
use mongodb::sync::Database;
use crate::leaderboard::LeaderboardItem;

use crate::tools::origin::*;

mod tools;

#[tokio::main]
async fn main() {

    let access_token: String = auth::get_access_token().await;
    let leaderboard: Vec<LeaderboardItem> = leaderboard::get_leaderboard_page(&access_token, 1).await;

    let client: Client = Client::with_uri_str("mongodb://127.0.0.1/ronin").unwrap();
    let database: Database = client.database("ronin");
    let collection: Collection<PVPBattleLog> = database.collection::<PVPBattleLog>("pvpbattlelogs");

    let options = IndexOptions::builder().unique(true).build();
    let index_model = IndexModel::builder().keys(doc! {"battle_uuid": 1u32}).options(options).build();
    collection.create_index(index_model, None).expect("Failed to create index!");

    let mut items_to_insert: Vec<PVPBattleLog> = vec![];

    for player in leaderboard {
        println!("Checking Leaderboard for:");
        println!("Rank: {}", player.topRank);
        println!("Name: {}", player.name);
        println!("ID: {}", player.userID);
        println!("Stars: {}", player.vstar);

        let url: String = f!("https://tracking.skymavis.com/origin/battle-history?type=pvp&client_id={player.userID}");

        let client = reqwest::Client::new();
        let result: Result<BattleLogResult, reqwest::Error> = client.get(url)
            .send()
            .await
            .unwrap()
            .json().await;

        let mut battles = result.unwrap().battles;

        items_to_insert.append(&mut battles);

        println!("Battles: {}\n====================", battles.len())

    }

    let count_before = collection.count_documents(None, None).unwrap();

    let insert_options = InsertManyOptions::builder().ordered(false).build();
    collection.insert_many(items_to_insert, insert_options).ok();

    let count_after = collection.count_documents(None, None).unwrap();

    let new_inserts = count_after - count_before;

    println!("Added {} battle logs to the database!", new_inserts);



}