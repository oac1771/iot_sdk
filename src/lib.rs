pub mod central;

pub use btleplug::{platform::Peripheral as PlatformPeripheral, api::{Characteristic, Peripheral, CharPropFlags}};
pub use uuid::Uuid;
