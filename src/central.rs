use btleplug::{
    api::{
        Central as ApiCentral, CentralEvent, CharPropFlags, Manager, Peripheral, ScanFilter,
        ValueNotification, PeripheralProperties
    },
    platform::{Adapter, Manager as PlatformManager, Peripheral as PlatformPeripheral},
};
use futures::{Stream, StreamExt, pin_mut};
use uuid::Uuid;

pub struct Central(Adapter);

impl Central {
    pub async fn new() -> Result<Self, Error> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;
        let central = adapters.into_iter().nth(0).ok_or(Error::AdapterNotFound)?;

        Ok(Self(central))
    }

    async fn peripheral_properties(&self) -> Result<impl Stream<Item = PeripheralProperties>, Error> {
        let peripherals = self.events().await?.filter_map(|central_event| async {
            if let CentralEvent::DeviceUpdated(id) = central_event {
                let peripheral = self.0.peripheral(&id).await.unwrap();
                let properties = peripheral
                    .properties()
                    .await
                    .unwrap()
                    .ok_or_else(|| Error::PeripheralPropertiesNotFound)
                    .unwrap();
                Some(properties)
            } else {
                None
            }
        });
        Ok(peripherals)
    }

    pub async fn find_peripheral(&self, local_name: &str) -> Result<(), Error> {
        let peripherals = self.peripherals().await?;

        pin_mut!(peripherals);

        let peripheral = loop {
            if let Some(peripheral) = peripherals.next().await {
                let properties = peripheral
                    .properties()
                    .await?
                    .ok_or_else(|| Error::PeripheralPropertiesNotFound)?;

                if properties
                    .local_name
                    .ok_or_else(|| Error::LocalNameNotFound)?
                    == local_name
                {
                    break peripheral;
                }
            }
        };

        peripheral.connect().await?;
        peripheral.discover_services().await?;

        Ok(())
    }

    pub async fn subscribe(
        &self,
        peripheral: &PlatformPeripheral,
        characteristic_uuid: Uuid,
    ) -> Result<impl Stream<Item = ValueNotification>, Error> {
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics
            .iter()
            .find(|c| Uuid::from_bytes(*c.uuid.as_bytes()) == characteristic_uuid)
            .ok_or_else(|| Error::CharacteristicNotFound)?;

        if !characteristic
            .properties
            .iter()
            .any(|c| c == CharPropFlags::NOTIFY)
        {
            return Err(Error::CharacteristicDoesNotSupportNotify);
        }

        peripheral.subscribe(characteristic).await?;

        let stream = peripheral.notifications().await?.filter(move |n| {
            let notif_uuid = Uuid::from_bytes(*n.uuid.as_bytes());
            let characteristic_uuid = characteristic_uuid;

            async move { notif_uuid == characteristic_uuid }
        });

        Ok(stream)
    }

    pub async fn write(
        &self,
        peripheral: &PlatformPeripheral,
        characteristic_uuid: Uuid,
        data: &[u8],
    ) -> Result<(), Error> {
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics
            .iter()
            .find(|c| Uuid::from_bytes(*c.uuid.as_bytes()) == characteristic_uuid)
            .ok_or_else(|| Error::CharacteristicNotFound)?;

        if !characteristic
            .properties
            .iter()
            .any(|c| c == CharPropFlags::WRITE)
        {
            return Err(Error::CharacteristicDoesNotSupportWrite);
        }

        peripheral
            .write(
                characteristic,
                data,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await?;

        Ok(())
    }

    pub async fn read(
        &self,
        peripheral: &PlatformPeripheral,
        characteristic_uuid: Uuid,
    ) -> Result<Vec<u8>, Error> {
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics
            .iter()
            .find(|c| Uuid::from_bytes(*c.uuid.as_bytes()) == characteristic_uuid)
            .ok_or_else(|| Error::CharacteristicNotFound)?;

        if !characteristic
            .properties
            .iter()
            .any(|c| c == CharPropFlags::READ)
        {
            return Err(Error::CharacteristicDoesNotSupportRead);
        }

        let result = peripheral.read(characteristic).await?;

        Ok(result)
    }

    async fn events(&self) -> Result<impl Stream<Item = CentralEvent>, Error> {
        let events = self.0.events().await?;
        self.0.start_scan(ScanFilter::default()).await?;

        Ok(events)
    }

    async fn peripherals(&self) -> Result<impl Stream<Item = PlatformPeripheral>, Error> {
        let peripherals = self.events().await?.filter_map(|central_event| async {
            if let CentralEvent::DeviceUpdated(id) = central_event {
                let peripheral = self.0.peripheral(&id).await.unwrap();
                Some(peripheral)
            } else {
                None
            }
        });
        Ok(peripherals)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{source}")]
    BtlePlug {
        #[from]
        source: btleplug::Error,
    },

    #[error("AdapterNotFound")]
    AdapterNotFound,

    #[error("PeripheralPropertiesNotFound")]
    PeripheralPropertiesNotFound,

    #[error("LocalNameNotFound")]
    LocalNameNotFound,

    #[error("CharacteristicNotFound")]
    CharacteristicNotFound,

    #[error("CharacteristicDoesNotSupportRead")]
    CharacteristicDoesNotSupportRead,

    #[error("CharacteristicDoesNotSupportWrite")]
    CharacteristicDoesNotSupportWrite,

    #[error("CharacteristicDoesNotSupportNotify")]
    CharacteristicDoesNotSupportNotify,
}
