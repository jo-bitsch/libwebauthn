use async_trait::async_trait;
use hidapi::DeviceInfo;
use hidapi::HidApi;
use std::fmt;
#[allow(unused_imports)]
use tracing::{debug, info, instrument};

#[cfg(feature = "virtual-hid-device")]
use solo::SoloVirtualKey;

use super::channel::HidChannel;
use super::Hid;

use crate::transport::error::TransportError;
use crate::transport::Device;
use crate::webauthn::error::Error;

#[derive(Debug)]
// SoloVirtualKey is not clone-able, but in test-mode we don't care
#[cfg_attr(not(feature = "virtual-hid-device"), derive(Clone))]
pub struct HidDevice {
    pub backend: HidBackendDevice,
}

#[derive(Debug)]
// SoloVirtualKey is not clone-able, but in test-mode we don't care
#[cfg_attr(not(feature = "virtual-hid-device"), derive(Clone))]
pub enum HidBackendDevice {
    HidApiDevice(DeviceInfo),
    #[cfg(feature = "virtual-hid-device")]
    VirtualDevice(SoloVirtualKey),
}

impl From<&DeviceInfo> for HidDevice {
    fn from(hidapi_device: &DeviceInfo) -> Self {
        Self {
            backend: HidBackendDevice::HidApiDevice(hidapi_device.clone()),
        }
    }
}

impl fmt::Display for HidDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.backend {
            HidBackendDevice::HidApiDevice(dev) => write!(
                f,
                "{:} {:} (r{:?})",
                dev.manufacturer_string().unwrap(),
                dev.product_string().unwrap(),
                dev.release_number()
            ),
            #[cfg(feature = "virtual-hid-device")]
            HidBackendDevice::VirtualDevice(dev) => dev.fmt(f),
        }
    }
}

pub(crate) fn get_hidapi() -> Result<HidApi, Error> {
    HidApi::new().or(Err(Error::Transport(TransportError::TransportUnavailable)))
}

#[cfg(feature = "virtual-hid-device")]
#[instrument]
pub async fn list_devices() -> Result<Vec<HidDevice>, Error> {
    info!("Faking device list, returning virtual device");
    Ok(vec![HidDevice::new_virtual()])
}

#[cfg(not(feature = "virtual-hid-device"))]
#[instrument]
pub async fn list_devices() -> Result<Vec<HidDevice>, Error> {
    let devices: Vec<_> = get_hidapi()?
        .device_list()
        .filter(|device| device.usage_page() == 0xF1D0)
        .filter(|device| device.usage() == 0x0001)
        .map(|device| device.into())
        .collect();
    info!({ count = devices.len() }, "Listing available HID devices");
    debug!(?devices);
    Ok(devices)
}

impl HidDevice {
    #[cfg(feature = "virtual-hid-device")]
    pub fn new_virtual() -> Self {
        let solo = SoloVirtualKey::default();
        Self {
            backend: HidBackendDevice::VirtualDevice(solo),
        }
    }
}

#[async_trait]
impl<'d> Device<'d, Hid, HidChannel<'d>> for HidDevice {
    async fn channel(&'d mut self) -> Result<HidChannel<'d>, Error> {
        let channel = HidChannel::new(self).await?;
        Ok(channel)
    }

    // async fn supported_protocols(&mut self) -> Result<SupportedProtocols, Error> {
    //     let channel = self.channel().await?;
    //     channel.supported_protocols().await
    // }
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "hid-device-tests")]
    #[tokio::test]
    async fn test_supported_protocols() {
        use super::HidDevice;
        use crate::transport::channel::Channel;
        use crate::transport::Device;

        let mut device = HidDevice::new_virtual();
        let channel = device.channel().await.unwrap();

        let protocols = channel.supported_protocols().await.unwrap();

        assert!(protocols.u2f);
        assert!(protocols.fido2);
    }
}
