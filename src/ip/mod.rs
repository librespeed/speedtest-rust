use serde::{Deserialize, Serialize};

pub mod ip_info;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct IPInfo {
    pub processedString : String,
    pub rawIspInfo : RawISPInfo
}

impl IPInfo {
    pub fn empty () -> Self {
        IPInfo {
            processedString : "".to_string(),
            rawIspInfo : RawISPInfo {
                ip : "".to_string(),
                hostname : "".to_string(),
                city : "".to_string(),
                region : "".to_string(),
                country : "".to_string(),
                location : "".to_string(),
                organization : "".to_string(),
                postal : "".to_string(),
                timezone : "".to_string(),
                readme : None
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RawISPInfo {
    pub ip : String,
    pub hostname : String,
    pub city : String,
    pub region : String,
    pub country : String,
    #[serde(rename = "loc")]
    pub location : String,
    #[serde(rename = "org")]
    pub organization : String,
    pub postal : String,
    pub timezone : String,
    pub readme : Option<String>
}