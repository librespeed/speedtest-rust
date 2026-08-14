use std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use log::info;
use tokio::sync::Mutex;
use crate::database::Database;
use crate::results::TelemetryData;

pub struct MemoryDB {
    pub records : DashMap<String,TelemetryData>
}

impl MemoryDB {

    pub fn init() -> Arc<Mutex<MemoryDB>> {
        info!("Database in-memory initialized successfully");
        Arc::new(Mutex::new(MemoryDB {records : DashMap::new()}))
    }

}

#[async_trait]
impl Database for MemoryDB {
    async fn insert(&mut self, data: TelemetryData) -> std::io::Result<()> {
        self.records.insert(data.uuid.clone(),data);
        if self.records.len() > 100 {
            if let Some(key) = self.records.iter().next().map(|entry| entry.key().clone()) {
                self.records.remove(&key);
            }
        }
        Ok(())
    }

    async fn fetch_by_uuid(&mut self, uuid: &str) -> std::io::Result<Option<TelemetryData>> {
        Ok(self.records.get(uuid).as_deref().cloned())
    }

    async fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let data : Vec<TelemetryData> = self.records.iter().map(|item| item.value().clone()).collect();
        Ok(data)
    }
}