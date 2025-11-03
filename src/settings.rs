use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BufferSettings {
    pub capacity: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub buffer: BufferSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("Settings"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn buffer_capacity(&self) -> usize {
        self.buffer.capacity
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            buffer: BufferSettings {
                capacity: 32 * 1024 * 1024, // 32 MB default
            },
        }
    }
}
