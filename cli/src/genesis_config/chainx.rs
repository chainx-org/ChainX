// Copyright 2018-2020 Chainpool.

use super::*;
use hex::FromHex;

const GENESIS_NODE_COUNT: usize = 29;
// 3 team + 5 council
const TEAM_COUNCIL_COUNT: usize = 8;

// (account, session_key, endowed, name, url, memo)
type IntentionConfig = (AccountId, AuthorityId, u64, Vec<u8>, Vec<u8>, Vec<u8>);
type GenesisNodeEntry = (
    AccountId,
    AuthorityId,
    u64,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
);

fn hex(account: &str) -> [u8; 32] {
    <[u8; 32] as FromHex>::from_hex(account).unwrap()
}

#[derive(Debug, Deserialize)]
struct CsvGenesisNodeEntry {
    account_id: String,
    session_key: String,
    endowed: f64,
    name: String,
    url: String,
    about: String,
    hot_entity: String,
    cold_entity: String,
}

pub fn load_genesis_node(csv: &[u8]) -> Result<Vec<GenesisNodeEntry>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(csv);
    let mut res = Vec::with_capacity(GENESIS_NODE_COUNT);
    for result in reader.deserialize() {
        let record: CsvGenesisNodeEntry = result?;

        let account_id = hex(&record.account_id).unchecked_into();
        let authority_key = hex(&record.session_key).unchecked_into();

        let endowed = (record.endowed * 10_u64.pow(u32::from(PCX_PRECISION)) as f64) as u64;
        let node_name = record.name.into_bytes();
        let node_url = record.url.into_bytes();
        let memo = record.about.into_bytes();
        let get_entity = |entity: String| {
            if entity.is_empty() {
                None
            } else {
                Some(Vec::from_hex(&entity).unwrap())
            }
        };
        let hot_key = get_entity(record.hot_entity);
        let cold_key = get_entity(record.cold_entity);
        res.push((
            account_id,
            authority_key,
            endowed,
            node_name,
            node_url,
            memo,
            hot_key,
            cold_key,
        ));
    }
    Ok(res)
}

#[derive(Debug, Deserialize)]
struct CsvTeamCouncilEntry {
    account_id: String,
}

pub fn load_team_council(csv: &[u8]) -> Result<Vec<AccountId>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(csv);
    let mut res = Vec::with_capacity(TEAM_COUNCIL_COUNT);
    for result in reader.deserialize() {
        let record: CsvTeamCouncilEntry = result?;
        let account_id = hex(&record.account_id).unchecked_into();
        res.push(account_id);
    }
    Ok(res)
}

pub fn bootstrap_intentions_config(genesis_node_info: &[GenesisNodeEntry]) -> Vec<IntentionConfig> {
    genesis_node_info
        .iter()
        .map(|(account_id, authority_id, value, name, url, memo, _, _)| {
            (
                account_id.clone(),
                authority_id.clone(),
                *value,
                name.clone(),
                url.clone(),
                memo.clone(),
            )
        })
        .collect()
}

pub fn bootstrap_trustee_intentions_config(
    genesis_node_info: &[GenesisNodeEntry],
) -> Vec<(AccountId, Vec<u8>, Vec<u8>)> {
    genesis_node_info
        .iter()
        .filter(|(_, _, _, _, _, _, hot_entity, cold_entity)| {
            hot_entity.is_some() && cold_entity.is_some()
        })
        .map(|(account_id, _, _, _, _, _, hot_entity, cold_entity)| {
            (
                account_id.clone(),
                hot_entity.clone().unwrap(),
                cold_entity.clone().unwrap(),
            )
        })
        .collect()
}
