use std::collections::HashMap;
use sha2::{Digest, Sha256};
use crate::database::generate_uuid;

const SECRET_KEY : &str = "BxKoUjZB6Pg6ymaeITnXudFa3QwNS1yz7JveTb8IGzFGh1xy98QdUChTy4K7P5Of";
const ID_PREFIX : &str = "_session";
const COOKIE_MA : i32 = 3600;

pub fn make_cookie(path : &str) -> String {
    let cookie_id = format!("{}{}",ID_PREFIX,generate_uuid());
    let sign_data = format!("{}{}{}",cookie_id,COOKIE_MA,SECRET_KEY);
    let cookie_sign = sha256_hash(&sign_data);
    format!("token={},{}; Path={}; Max-Age={}; HttpOnly; SameSite=Strict",cookie_id,cookie_sign,path,COOKIE_MA)
}

pub fn make_discard_cookie (path : &str) -> String {
    format!("token=deleted; path={}; expires=Thu, 01 Jan 1970 00:00:00 GMT",path)
}

pub fn validate_cookie(cookie_data : Option<&String>) -> bool {
    if let Some(cookie_data) = cookie_data {
        let cookie_parts : HashMap<&str,&str> = cookie_data.split(';').map(|s| s.split_at(s.find('=').unwrap())).map(|(key, val)| (key.trim(), &val[1..])).collect();
        let cookie_token = cookie_parts.get("token").unwrap_or(&"");
        let mut split_token = cookie_token.splitn(2,',');
        let cookie_id = split_token.next().unwrap_or("");
        let signature = split_token.next().unwrap_or("");
        let sign_data = format!("{}{}{}",cookie_id,COOKIE_MA,SECRET_KEY);
        let cookie_sign = sha256_hash(&sign_data);
        signature == cookie_sign
    } else {
        false
    }
}

fn sha256_hash(input : &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hash_string = format!("{:x}",result);
    hash_string
}