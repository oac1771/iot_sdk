pub mod central;

pub use btleplug::{platform::Peripheral as PlatformPeripheral, api::{Characteristic, Peripheral, CharPropFlags, ValueNotification}};
pub use uuid::Uuid;
