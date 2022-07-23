pub mod origin {
    pub mod auth {
        use std::collections::HashMap;

        use serde::Deserialize;
        use sha2::{Digest, Sha256};
        use sha2::digest::Update;

        const ORIGIN_AUTH_EMAIL: &str = "bierkoenig@scholar.axie.icu";
        const ORIGIN_AUTH_PASSWORD: &str = "S94amJPp";

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AuthResponse {
            access_token: String,
        }

        fn hash_password(password: &str) -> String {
            let mut hasher = Sha256::new();
            Update::update(&mut hasher, password.as_bytes());
            return format!("{:x}", hasher.finalize());
        }

        pub async fn get_access_token() -> String {
            let mut map = HashMap::new();
            map.insert("email", ORIGIN_AUTH_EMAIL);
            let hashed_password = hash_password(ORIGIN_AUTH_PASSWORD);
            map.insert("password", hashed_password.as_str());

            let client = reqwest::Client::new();
            let request: Result<AuthResponse, reqwest::Error> = client.post("https://athena.skymavis.com/v1/rpc/auth/login")
                .json(&map)
                .send()
                .await
                .unwrap()
                .json().await;

            match request {
                Ok(res) => res.access_token,
                Err(e) => panic!("{}", e)
            }
        }
    }

    pub mod leaderboard {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Leaderboard {
            _items: Vec<LeaderboardItem>,
        }

        #[derive(Deserialize, Clone)]
        pub struct LeaderboardItem {
            pub userID: String,
            pub name: String,
            pub rank: String,
            pub tier: u32,
            pub topRank: u32,
            pub vstar: u32,
        }

        pub async fn get_leaderboard_page(access_token: &String, page: u32) -> Vec<LeaderboardItem> {
            let offset = if page <= 0 { 0 } else { page - 1 } * 99;

            let mut request_url = "https://game-api-origin.skymavis.com/v2/users/me/seasons/current/leaderboards?limit=100&is_self_included=False&offset=".to_owned();
            request_url.push_str(&offset.to_string());

            let client = reqwest::Client::new();
            let result: Result<Leaderboard, reqwest::Error> = client.get(request_url)
                .header("User-Agent", "")
                .bearer_auth(&access_token)
                .send()
                .await
                .unwrap()
                .json().await;
            let mut items = result.unwrap()._items;
            items.retain(|i| {
                i.userID != "1ec9eb6f-896c-682f-a60c-19f2a53791d9".to_string()
            });
            items
        }
    }
}

pub mod types {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};

    pub type FighterGene = String;
    pub type Rune = String;

    #[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum BodyPart {
        Eyes,
        Mouth,
        Ears,
        Horn,
        Back,
        Tail,
    }

    pub type Charm = String;
    pub type Charms = HashMap<BodyPart, Option<Charm>>;

    #[derive(Serialize_repr, Deserialize_repr)]
    #[repr(u8)]
    #[derive(Debug)]
    pub enum AxieType {
        Free = 0,
        Owned = 1,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    pub struct Fighter {
        pub gene: FighterGene,
        pub axie_id: usize,
        pub axie_type: AxieType,
        pub runes: Option<Vec<Rune>>,
        pub charms: Charms,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum RewardResult {
        Win,
        Lose,
        Tie
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum ItemId {
        Exp,
        Moonshard,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RewardItem {
        pub item_id: ItemId,
        pub quantity: ItemQuantity,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Reward {
        pub user_id: ClientId,
        pub new_vstar: VStar,
        pub old_vstar: VStar,
        pub result: RewardResult,
        pub items: Option<Vec<RewardItem>>,
    }

    #[derive(Serialize_repr, Deserialize_repr)]
    #[repr(u8)]
    #[derive(Debug)]
    pub enum UserRankTier {
        Zero = 0,
        One = 1,
        Two = 2,
        Three = 3,
        Four = 4,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum UserRankDivision {
        Challenger,
        Dragon,
        Tiger,
        Bear,
        Wolf,
        Boar,
        Hare,
        Chick,
        Egg,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct UserRank {
        pub division: UserRankDivision,
        pub tier: UserRankTier,
    }

    pub type VStar = usize;
    pub type ItemQuantity = usize;
    pub type BattleId = String;
    pub type ClientId = String;
    pub type TeamId = usize;
    pub type BattleTimestamp = i64;
    pub type FighterTeam = Vec<Fighter>;
    pub type Rewards = Vec<Reward>;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PVPBattleLog {
        pub battle_uuid: BattleId,
        pub client_ids: Vec<ClientId>,
        pub team_ids: Vec<TeamId>,
        pub created_at: BattleTimestamp,
        pub winner: u8,
        pub battle_type: u8,
        pub first_client_fighters: FighterTeam,
        pub second_client_fighters: FighterTeam,
        pub rewards: Rewards,
        pub user_ranks: Vec<UserRank>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BattleLogResult {
        pub battles: Vec<PVPBattleLog>,
    }
}