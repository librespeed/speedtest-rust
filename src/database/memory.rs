use std::collections::HashMap;
use crate::database::Database;
use crate::results::TelemetryData;

pub struct MemoryDB {
    pub records : HashMap<String,TelemetryData>
}

pub fn init() -> HashMap<String,TelemetryData> {
    HashMap::new()
}

impl Database for MemoryDB {
    fn insert(&mut self, data: TelemetryData) -> std::io::Result<()> {
        self.records.insert(data.uuid.clone(),data);
        if self.records.len() > 100 {
            if let Some(key) = self.records.keys().next().cloned() {
                self.records.remove(&key);
            }
        }
        Ok(())
    }

    fn fetch_by_uuid(&mut self, uuid: &str) -> std::io::Result<Option<TelemetryData>> {
        Ok(self.records.get(uuid).cloned())
    }

    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let data : Vec<TelemetryData> = self.records.values().cloned().collect();
        Ok(data)
    }
}