pub mod database {
    use mongodb::bson::DateTime;
    use crate::contracts::contracts::ContractType;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    pub struct Transfer {
        pub from: String,
        pub to: String,
        pub token: String,
        pub value_or_token_id: String,
        pub created_at: DateTime,
        pub block: u64,
        pub transaction_id: String,
        pub erc: ContractType
    }

    impl Transfer {
        pub fn empty() -> Transfer {
            Transfer {
                from: "0x0000000000000000000000000000000000000000".to_string(),
                to: "0x0000000000000000000000000000000000000000".to_string(),
                token: "0x0000000000000000000000000000000000000000".to_string(),
                value_or_token_id: "0".to_string(),
                created_at: DateTime::from_millis(chrono::Utc::now().timestamp() * 1000),
                block: 0u64,
                transaction_id: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                erc: ContractType::Unknown
            }
        }
    }
}

pub mod contracts {
    use std::collections::HashMap;
    use serde::{Serialize, Deserialize};

    pub type ContractList = HashMap<&'static str, Contract>;

    #[derive(Serialize, Deserialize)]
    pub enum ContractType {
        ERC20,
        ERC721,
        Unknown
    }

    #[derive(Serialize, Deserialize)]
    pub struct Contract {
        pub name: &'static str,
        pub decimals: usize,
        pub erc: ContractType,
    }

    pub fn default() -> ContractList {
        let mut map: ContractList = HashMap::new();

        map.insert("0xc99a6a985ed2cac1ef41640596c5a5f9f4e19ef5", Contract {
            name: "WETH",
            decimals: 18,
            erc: ContractType::ERC20
        });

        map.insert("0xed4a9f48a62fb6fdcfb45bb00c9f61d1a436e58c", Contract {
            name: "AXS",
            decimals: 18,
            erc: ContractType::ERC20,
        });

        map.insert("0xa8754b9fa15fc18bb59458815510e40a12cd2014", Contract {
            name: "SLP",
            decimals: 0,
            erc: ContractType::ERC20,
        });

        map.insert("0x173a2d4fa585a63acd02c107d57f932be0a71bcc", Contract {
            name: "AEC",
            decimals: 0,
            erc: ContractType::ERC20,
        });

        map.insert("0x0b7007c13325c48911f73a2dad5fa5dcbf808adc", Contract {
            name: "USDC",
            decimals: 18,
            erc: ContractType::ERC20,
        });

        map.insert("0xe514d9deb7966c8be0ca922de8a064264ea6bcd4", Contract {
            name: "WRON",
            decimals: 18,
            erc: ContractType::ERC20,
        });

        map.insert("0x32950db2a7164ae833121501c797d79e7b79d74c", Contract {
            name: "AXIE",
            decimals: 0,
            erc: ContractType::ERC721,
        });

        map.insert("0x8c811e3c958e190f5ec15fb376533a3398620500", Contract {
            name: "LAND",
            decimals: 0,
            erc: ContractType::ERC721,
        });

        map.insert("0xa96660f0e4a3e9bc7388925d245a6d4d79e21259", Contract {
            name: "ITEM",
            decimals: 0,
            erc: ContractType::ERC721,
        });

        map
    }
}

pub mod events {
    use web3::ethabi::{Event, EventParam, ParamType};

    // pub fn erc_20_burn() -> Event {
    //     Event {
    //         name: "Burn".to_string(),
    //         inputs: vec![
    //             EventParam {
    //                 name: "_value".to_string(),
    //                 kind: ParamType::Uint(256),
    //                 indexed: true,
    //             }
    //         ],
    //         anonymous: false,
    //     }
    // }

    pub fn erc_20_transfer() -> Event {
        Event {
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
                    name: "_value".to_string(),
                    kind: ParamType::Uint(256),
                    indexed: false,
                },
            ],
            anonymous: false,
        }
    }

    pub fn erc_721_transfer() -> Event {
        Event {
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
        }
    }
}