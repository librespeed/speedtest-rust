use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/* ip info country/asn database model */

fn deserialize_mmdb_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::String(value) => value,
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => String::new(),
    })
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct MMDBResult {
    #[serde(
        default,
        deserialize_with = "deserialize_mmdb_string",
        alias = "autonomous_system_number"
    )]
    pub asn: String,
    #[serde(default, alias = "autonomous_system_organization")]
    pub as_name: String,
    #[serde(default)]
    pub as_domain: String,
    #[serde(default)]
    pub continent: String,
    #[serde(default)]
    pub continent_name: String,
    #[serde(default, alias = "country_code")]
    pub country: String,
    #[serde(default)]
    pub country_name: String,
}

impl MMDBResult {
    pub fn from_value(value: &Value) -> Option<Self> {
        let result = MMDBResult {
            asn: first_string(
                value,
                &[&["asn"], &["autonomous_system_number"], &["as_number"]],
            ),
            as_name: first_string(
                value,
                &[
                    &["as_name"],
                    &["autonomous_system_organization"],
                    &["organization"],
                ],
            ),
            as_domain: first_string(value, &[&["as_domain"], &["domain"]]),
            continent: first_string(value, &[&["continent"], &["continent", "code"]]),
            continent_name: first_string(
                value,
                &[&["continent_name"], &["continent", "names", "en"]],
            ),
            country: normalize_country_code(&first_string(
                value,
                &[
                    &["country"],
                    &["country", "iso_code"],
                    &["country_code"],
                    &["registered_country", "iso_code"],
                    &["registered_country", "code"],
                ],
            )),
            country_name: first_string(
                value,
                &[
                    &["country_name"],
                    &["country", "names", "en"],
                    &["registered_country", "names", "en"],
                ],
            ),
        };

        if result.asn.is_empty()
            && result.as_name.is_empty()
            && result.as_domain.is_empty()
            && result.continent.is_empty()
            && result.continent_name.is_empty()
            && result.country.is_empty()
            && result.country_name.is_empty()
        {
            None
        } else {
            Some(result)
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct MMDBCityResult {
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub country_name: String,
    #[serde(default)]
    pub postal: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

impl MMDBCityResult {
    pub fn from_value(value: &Value) -> Option<Self> {
        let result = MMDBCityResult {
            city: first_string(value, &[&["city"], &["city", "names", "en"]]),
            region: first_string(
                value,
                &[
                    &["region"],
                    &["subdivisions", "0", "names", "en"],
                    &["subdivisions", "0", "iso_code"],
                ],
            ),
            country: normalize_country_code(&first_string(
                value,
                &[
                    &["country"],
                    &["country", "iso_code"],
                    &["country_code"],
                    &["registered_country", "iso_code"],
                ],
            )),
            country_name: first_string(
                value,
                &[
                    &["country_name"],
                    &["country", "names", "en"],
                    &["registered_country", "names", "en"],
                ],
            ),
            postal: first_string(value, &[&["postal"], &["postal", "code"]]),
            timezone: first_string(
                value,
                &[&["timezone"], &["time_zone"], &["location", "time_zone"]],
            ),
            latitude: first_latitude(value, &[&["latitude"], &["location", "latitude"]]),
            longitude: first_longitude(value, &[&["longitude"], &["location", "longitude"]]),
        };

        if result.city.is_empty()
            && result.region.is_empty()
            && result.country.is_empty()
            && result.country_name.is_empty()
            && result.postal.is_empty()
            && result.timezone.is_empty()
            && result.latitude.is_none()
            && result.longitude.is_none()
        {
            None
        } else {
            Some(result)
        }
    }
}

fn first_string(value: &Value, paths: &[&[&str]]) -> String {
    for path in paths {
        if let Some(found) = value_at(value, path) {
            match found {
                Value::String(found) if !found.trim().is_empty() => {
                    return found.trim().to_string();
                }
                Value::Number(found) => return found.to_string(),
                Value::Bool(found) => return found.to_string(),
                _ => {}
            }
        }
    }
    String::new()
}

fn first_latitude(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    for path in paths {
        if let Some(found) = value_at(value, path) {
            if let Some(number) = value_as_f64(found) {
                if number.is_finite() && (-90.0..=90.0).contains(&number) {
                    return Some(number);
                }
            }
        }
    }
    None
}

fn first_longitude(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    for path in paths {
        if let Some(found) = value_at(value, path) {
            if let Some(number) = value_as_f64(found) {
                if number.is_finite() && (-180.0..=180.0).contains(&number) {
                    return Some(number);
                }
            }
        }
    }
    None
}

fn value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

fn normalize_country_code(country: &str) -> String {
    let country = country.trim();
    if country.is_empty() {
        String::new()
    } else {
        country.to_ascii_uppercase()
    }
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for (index, key) in path.iter().enumerate() {
        current = match current {
            Value::Object(object) => object.get(*key)?,
            Value::Array(array) if *key == "0" => array.first()?,
            _ => return None,
        };
        if index + 1 == path.len() {
            return Some(current);
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn city_coordinates_must_be_geographically_valid() {
        let valid = json!({
            "location": {"latitude": 34.0522, "longitude": -118.2437}
        });
        let parsed = MMDBCityResult::from_value(&valid).expect("valid coordinates");
        assert_eq!(parsed.latitude, Some(34.0522));
        assert_eq!(parsed.longitude, Some(-118.2437));

        let invalid = json!({
            "location": {"latitude": 120.0, "longitude": 250.0}
        });
        assert!(MMDBCityResult::from_value(&invalid).is_none());
    }
}
