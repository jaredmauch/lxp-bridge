use crate::prelude::*;

use eg4::{
    inverter::WaitForReply,
    packet::{DeviceFunction, TranslatedData},
};

use crate::coordinator::Channels;
use crate::config;

pub struct ReadHold {
    channels: Channels,
    inverter: config::Inverter,
    register: u16,
    count: u16,
}

impl ReadHold {
    pub fn new<U>(channels: Channels, inverter: config::Inverter, register: U, count: u16) -> Self
    where
        U: Into<u16>,
    {
        Self {
            channels,
            inverter,
            register: register.into(),
            count,
        }
    }

    pub async fn run(&self) -> Result<Packet> {
        let packet = Packet::TranslatedData(TranslatedData {
            datalog: self.inverter.datalog().expect("datalog must be set for read_hold command"),
            device_function: DeviceFunction::ReadHold,
            inverter: self.inverter.serial().expect("serial must be set for read_hold command"),
            register: self.register,
            values: vec![self.count as u8, 0],
        });

        let mut receiver = self.channels.from_inverter.subscribe();

        if self
            .channels
            .to_inverter
            .send(eg4::inverter::ChannelData::Packet(packet.clone()))
            .is_err()
        {
            bail!("send(to_inverter) failed - channel closed?");
        }

        let packet = receiver.wait_for_reply(&packet).await?;
        Ok(packet)
    }
}
