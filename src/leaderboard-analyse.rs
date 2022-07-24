mod tools;

use tools::database::{MongoDb, Options};
use crate::tools::types::PVPBattleLog;

#[tokio::main]
async fn main() {

    let db_options = Options {
        client_uri: "mongodb://127.0.0.1".to_string(),
        database: "test".to_string()
    };

    struct Statistic {
        total_battlelogs:i32,

    }

    let db = MongoDb::new(db_options).await;
    let battlelogs = db.database.collection::<PVPBattleLog>("axie");

    println!("huhu :D")
}