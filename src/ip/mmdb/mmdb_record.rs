use serde::Deserialize;

/*ip info country asn database model*/

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MMDBResult {
    pub asn: String,
    pub as_name: String,
    pub as_domain: String,
    pub continent: String,
    pub continent_name: String,
    pub country: String,
    pub country_name: String,
}

#[derive(Deserialize)]
pub struct MMDBRecord<'a> {
    asn: Option<&'a str>,
    as_name: Option<&'a str>,
    as_domain: Option<&'a str>,
    continent: Option<&'a str>,
    continent_name: Option<&'a str>,
    country: Option<&'a str>,
    country_name: Option<&'a str>,
}

impl MMDBRecord<'_> {
    pub fn get_result(&self) -> MMDBResult {
        MMDBResult {
            asn: self.asn.unwrap_or_default().to_string(),
            as_name: self.as_name.unwrap_or_default().to_string(),
            as_domain: self.as_domain.unwrap_or_default().to_string(),
            continent: self.continent.unwrap_or_default().to_string(),
            continent_name: self.continent_name.unwrap_or_default().to_string(),
            country: self.country.unwrap_or_default().to_string(),
            country_name: self.country_name.unwrap_or_default().to_string(),
        }
    }
}