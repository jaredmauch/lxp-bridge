mod common;
use common::*;
use lxp_bridge::prelude::*;
use lxp_bridge::{lxp, mqtt};
use lxp_bridge::lxp::packet::{DeviceFunction, Packet, TranslatedData};
use lxp_bridge::lxp::inverter::Serial;
use lxp_bridge::coordinator::commands::read_inputs::ReadInputs;
use lxp_bridge::lxp::inverter::ChannelData;

#[tokio::test]
async fn happy_path() {
    common_setup();

    let inverter = Factory::inverter();
    let channels = Channels::new();

    let register = 0 as u16;
    let count = 40 as u16;

    let subject = coordinator::commands::read_inputs::ReadInputs::new(
        channels.clone(),
        inverter.clone(),
        register,
        count,
    );

    let reply = Packet::TranslatedData(lxp::packet::TranslatedData {
        datalog: inverter.datalog(),
        device_function: lxp::packet::DeviceFunction::ReadInput,
        inverter: inverter.serial(),
        register: 0,
        values: vec![0, 0],
    });

    let sf = async {
        let result = subject.run().await;
        assert_eq!(result?, reply.clone());
        Ok(())
    };

    let tf = async {
        channels.to_inverter.subscribe().recv().await?;
        channels
            .from_inverter
            .send(lxp::inverter::ChannelData::Packet(reply.clone()))?;
        Ok::<(), anyhow::Error>(())
    };

    futures::try_join!(tf, sf).unwrap();
}
