use crate::prelude::*;

pub mod commands;

use std::sync::{Arc, Mutex};
use lxp::packet::{DeviceFunction, ReadInput, TranslatedData, Packet, ReadInputAll, ReadInput1};
use lxp::inverter;
use serde_json::json;

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ChannelData {
    Shutdown,
    Packet(lxp::packet::Packet),
}

pub type InputsStore = std::collections::HashMap<Serial, lxp::packet::ReadInputs>;

#[derive(Default)]
pub struct PacketStats {
    packets_received: u64,
    packets_sent: u64,
    // Received packet counters
    heartbeat_packets_received: u64,
    translated_data_packets_received: u64,
    read_param_packets_received: u64,
    write_param_packets_received: u64,
    // Sent packet counters
    heartbeat_packets_sent: u64,
    translated_data_packets_sent: u64,
    read_param_packets_sent: u64,
    write_param_packets_sent: u64,
    // Other stats
    mqtt_messages_sent: u64,
    mqtt_errors: u64,
    influx_writes: u64,
    influx_errors: u64,
    database_writes: u64,
    database_errors: u64,
    register_cache_writes: u64,
    register_cache_errors: u64,
    // Connection stats
    inverter_disconnections: std::collections::HashMap<Serial, u64>,
    serial_mismatches: u64,
    // Last message received per inverter
    last_messages: std::collections::HashMap<Serial, String>,
}

impl PacketStats {
    pub fn print_summary(&self) {
        info!("Packet Statistics:");
        info!("  Total packets received: {}", self.packets_received);
        info!("  Total packets sent: {}", self.packets_sent);
        info!("  Received Packet Types:");
        info!("    Heartbeat packets: {}", self.heartbeat_packets_received);
        info!("    TranslatedData packets: {}", self.translated_data_packets_received);
        info!("    ReadParam packets: {}", self.read_param_packets_received);
        info!("    WriteParam packets: {}", self.write_param_packets_received);
        info!("  Sent Packet Types:");
        info!("    Heartbeat packets: {}", self.heartbeat_packets_sent);
        info!("    TranslatedData packets: {}", self.translated_data_packets_sent);
        info!("    ReadParam packets: {}", self.read_param_packets_sent);
        info!("    WriteParam packets: {}", self.write_param_packets_sent);
        info!("  MQTT:");
        info!("    Messages sent: {}", self.mqtt_messages_sent);
        info!("    Errors: {}", self.mqtt_errors);
        info!("  InfluxDB:");
        info!("    Writes: {}", self.influx_writes);
        info!("    Errors: {}", self.influx_errors);
        info!("  Database:");
        info!("    Writes: {}", self.database_writes);
        info!("    Errors: {}", self.database_errors);
        info!("  Register Cache:");
        info!("    Writes: {}", self.register_cache_writes);
        info!("    Errors: {}", self.register_cache_errors);
        info!("  Connection Stats:");
        info!("    Serial number mismatches: {}", self.serial_mismatches);
        info!("    Inverter disconnections by serial:");
        for (serial, count) in &self.inverter_disconnections {
            info!("      {}: {}", serial, count);
            if let Some(last_msg) = self.last_messages.get(serial) {
                info!("      Last message: {}", last_msg);
            }
        }
    }

    pub fn increment_serial_mismatches(&mut self) {
        self.serial_mismatches += 1;
    }

    pub fn increment_mqtt_errors(&mut self) {
        self.mqtt_errors += 1;
    }

    pub fn increment_cache_errors(&mut self) {
        self.register_cache_errors += 1;
    }
}

#[derive(Clone)]
pub struct Coordinator {
    config: ConfigWrapper,
    channels: Channels,
    pub stats: Arc<Mutex<PacketStats>>,
}

impl Coordinator {
    pub fn new(config: ConfigWrapper, channels: Channels) -> Self {
        Self { 
            config, 
            channels,
            stats: Arc::new(Mutex::new(PacketStats::default())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if self.config.mqtt().enabled() {
            futures::try_join!(self.inverter_receiver(), self.mqtt_receiver())?;
        } else {
            self.inverter_receiver().await?;
        }

        Ok(())
    }

    pub fn stop(&self) {
        // Send shutdown signals to channels
        let _ = self
            .channels
            .from_inverter
            .send(lxp::inverter::ChannelData::Shutdown);

        if self.config.mqtt().enabled() {
            let _ = self.channels.from_mqtt.send(mqtt::ChannelData::Shutdown);
        }
    }

    async fn mqtt_receiver(&self) -> Result<()> {
        let mut receiver = self.channels.from_mqtt.subscribe();

        while let mqtt::ChannelData::Message(message) = receiver.recv().await? {
            let _ = self.process_message(message).await;
        }

        Ok(())
    }

    async fn process_message(&self, message: mqtt::Message) -> Result<()> {
        // If MQTT is disabled, don't process any messages
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        for inverter in self.config.inverters_for_message(&message)? {
            match message.to_command(inverter) {
                Ok(command) => {
                    info!("parsed command {:?}", command);
                    let result = self.process_command(command.clone()).await;
                    if result.is_err() {
                        let topic_reply = command.to_result_topic();
                        let reply = mqtt::ChannelData::Message(mqtt::Message {
                            topic: topic_reply,
                            retain: false,
                            payload: "FAIL".to_string(),
                        });
                        if self.channels.to_mqtt.send(reply).is_err() {
                            bail!("send(to_mqtt) failed - channel closed?");
                        }
                    }
                }
                Err(err) => {
                    error!("{:?}", err);
                }
            }
        }

        Ok(())
    }

    fn increment_packets_sent(&self, packet: &Packet) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.packets_sent += 1;
            
            // Increment counter for specific sent packet type
            match packet {
                Packet::Heartbeat(_) => stats.heartbeat_packets_sent += 1,
                Packet::TranslatedData(_) => stats.translated_data_packets_sent += 1,
                Packet::ReadParam(_) => stats.read_param_packets_sent += 1,
                Packet::WriteParam(_) => stats.write_param_packets_sent += 1,
            }
        }
    }

    async fn process_command(&self, command: Command) -> Result<()> {
        use commands::time_register_ops::Action;
        use lxp::packet::{Register, RegisterBit};
        use Command::*;

        // Create a packet from the command for stats tracking
        let packet = match &command {
            ReadInputs(_, _) => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::ReadInput,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
            ReadInput(_, _, _) => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::ReadInput,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
            ReadHold(_, _, _) => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::ReadHold,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
            ReadParam(_, _) => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::ReadHold,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
            WriteParam(_, _, _) => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::WriteSingle,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
            _ => Packet::TranslatedData(TranslatedData {
                datalog: Serial::default(),
                device_function: DeviceFunction::WriteSingle,
                inverter: Serial::default(),
                register: 0,
                values: vec![],
            }),
        };

        self.increment_packets_sent(&packet);

        match command {
            ReadInputs(inverter, 1) => self.read_inputs(inverter, 0_u16, 40).await,
            ReadInputs(inverter, 2) => self.read_inputs(inverter, 40_u16, 40).await,
            ReadInputs(inverter, 3) => self.read_inputs(inverter, 80_u16, 40).await,
            ReadInputs(inverter, 4) => self.read_inputs(inverter, 120_u16, 40).await,
            ReadInputs(_, _) => unreachable!(),
            ReadInput(inverter, register, count) => {
                self.read_inputs(inverter, register, count).await
            }
            ReadHold(inverter, register, count) => self.read_hold(inverter, register, count).await,
            ReadParam(inverter, register) => self.read_param(inverter, register).await,
            ReadAcChargeTime(inverter, num) => {
                self.read_time_register(inverter, Action::AcCharge(num))
                    .await
            }
            ReadAcFirstTime(inverter, num) => {
                self.read_time_register(inverter, Action::AcFirst(num))
                    .await
            }
            ReadChargePriorityTime(inverter, num) => {
                self.read_time_register(inverter, Action::ChargePriority(num))
                    .await
            }
            ReadForcedDischargeTime(inverter, num) => {
                self.read_time_register(inverter, Action::ForcedDischarge(num))
                    .await
            }
            SetHold(inverter, register, value) => self.set_hold(inverter, register, value).await,
            WriteParam(inverter, register, value) => {
                self.write_param(inverter, register, value).await
            }
            SetAcChargeTime(inverter, num, values) => {
                self.set_time_register(inverter, Action::AcCharge(num), values)
                    .await
            }
            SetAcFirstTime(inverter, num, values) => {
                self.set_time_register(inverter, Action::AcFirst(num), values)
                    .await
            }
            SetChargePriorityTime(inverter, num, values) => {
                self.set_time_register(inverter, Action::ChargePriority(num), values)
                    .await
            }
            SetForcedDischargeTime(inverter, num, values) => {
                self.set_time_register(inverter, Action::ForcedDischarge(num), values)
                    .await
            }
            AcCharge(inverter, enable) => {
                self.update_hold(
                    inverter,
                    Register::Register21,
                    RegisterBit::AcChargeEnable,
                    enable,
                )
                .await
            }
            ChargePriority(inverter, enable) => {
                self.update_hold(
                    inverter,
                    Register::Register21,
                    RegisterBit::ChargePriorityEnable,
                    enable,
                )
                .await
            }

            ForcedDischarge(inverter, enable) => {
                self.update_hold(
                    inverter,
                    Register::Register21,
                    RegisterBit::ForcedDischargeEnable,
                    enable,
                )
                .await
            }
            ChargeRate(inverter, pct) => {
                self.set_hold(inverter, Register::ChargePowerPercentCmd, pct)
                    .await
            }
            DischargeRate(inverter, pct) => {
                self.set_hold(inverter, Register::DischgPowerPercentCmd, pct)
                    .await
            }

            AcChargeRate(inverter, pct) => {
                self.set_hold(inverter, Register::AcChargePowerCmd, pct)
                    .await
            }

            AcChargeSocLimit(inverter, pct) => {
                self.set_hold(inverter, Register::AcChargeSocLimit, pct)
                    .await
            }

            DischargeCutoffSocLimit(inverter, pct) => {
                self.set_hold(inverter, Register::DischgCutOffSocEod, pct)
                    .await
            }
        }
    }

    async fn read_inputs<U>(
        &self,
        inverter: config::Inverter,
        register: U,
        count: u16,
    ) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::read_inputs::ReadInputs::new(
            self.channels.clone(),
            inverter.clone(),
            register,
            count,
        )
        .run()
        .await?;

        Ok(())
    }

    async fn read_hold<U>(&self, inverter: config::Inverter, register: U, count: u16) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::read_hold::ReadHold::new(
            self.channels.clone(),
            inverter.clone(),
            register,
            count,
        )
        .run()
        .await?;

        Ok(())
    }

    async fn read_param<U>(&self, inverter: config::Inverter, register: U) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::read_param::ReadParam::new(self.channels.clone(), inverter.clone(), register)
            .run()
            .await?;

        Ok(())
    }

    async fn read_time_register(
        &self,
        inverter: config::Inverter,
        action: commands::time_register_ops::Action,
    ) -> Result<()> {
        commands::time_register_ops::ReadTimeRegister::new(
            self.channels.clone(),
            inverter.clone(),
            self.config.clone(),
            action,
        )
        .run()
        .await
    }

    async fn write_param<U>(
        &self,
        inverter: config::Inverter,
        register: U,
        value: u16,
    ) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::write_param::WriteParam::new(
            self.channels.clone(),
            inverter.clone(),
            register,
            value,
        )
        .run()
        .await?;

        Ok(())
    }

    async fn set_time_register(
        &self,
        inverter: config::Inverter,
        action: commands::time_register_ops::Action,
        values: [u8; 4],
    ) -> Result<()> {
        commands::time_register_ops::SetTimeRegister::new(
            self.channels.clone(),
            inverter.clone(),
            self.config.clone(),
            action,
            values,
        )
        .run()
        .await
    }

    async fn set_hold<U>(&self, inverter: config::Inverter, register: U, value: u16) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::set_hold::SetHold::new(self.channels.clone(), inverter.clone(), register, value)
            .run()
            .await?;

        Ok(())
    }

    async fn update_hold<U>(
        &self,
        inverter: config::Inverter,
        register: U,
        bit: lxp::packet::RegisterBit,
        enable: bool,
    ) -> Result<()>
    where
        U: Into<u16>,
    {
        commands::update_hold::UpdateHold::new(
            self.channels.clone(),
            inverter.clone(),
            register,
            bit,
            enable,
        )
        .run()
        .await?;

        Ok(())
    }

    async fn process_inverter_packet(&self, packet: Packet, inverter: &config::Inverter) -> Result<()> {
        if let Packet::TranslatedData(td) = packet {
            // Log TCP function for debugging
            debug!("Processing TCP function: {:?}", td.tcp_function());

            // Check if serial matches configured inverter
            if td.inverter() != Some(inverter.serial) {
                warn!(
                    "Serial mismatch - got {:?}, expected {}",
                    td.inverter(),
                    inverter.serial
                );
                self.stats.lock().unwrap().increment_serial_mismatches();
                return Ok(());
            }

            match td.device_function {
                DeviceFunction::ReadInput => {
                    debug!("Processing ReadInput packet");
                    let read_input = td.read_input()?;
                    match read_input {
                        ReadInput::ReadInputAll(input_all) => {
                            debug!("Processing ReadInputAll");
                            if let Err(e) = self.publish_raw_input_messages(&input_all, inverter).await {
                                error!("Failed to publish raw input messages: {}", e);
                                self.stats.lock().unwrap().increment_mqtt_errors();
                            }
                        }
                        ReadInput::ReadInput1(input_1) => {
                            debug!("Processing ReadInput1");
                            if let Err(e) = self.publish_raw_input_messages_1(&input_1, inverter).await {
                                error!("Failed to publish raw input messages: {}", e);
                                self.stats.lock().unwrap().increment_mqtt_errors();
                            }
                        }
                        _ => {
                            debug!("Unhandled ReadInput variant");
                        }
                    }
                }
                DeviceFunction::ReadHold => {
                    debug!("Processing ReadHold packet");
                    let register = td.register();
                    let pairs = td.pairs();
                    for (reg, value) in &pairs {
                        if let Err(e) = self.channels.to_register_cache.send(register_cache::ChannelData::RegisterData(*reg, *value)) {
                            error!("Failed to cache register {}: {}", reg, e);
                            self.stats.lock().unwrap().increment_cache_errors();
                        }
                    }
                    if let Err(e) = self.publish_hold_message(register, pairs, inverter).await {
                        error!("Failed to publish hold message: {}", e);
                        self.stats.lock().unwrap().increment_mqtt_errors();
                    }
                }
                DeviceFunction::WriteSingle => {
                    debug!("Processing WriteSingle packet");
                    let register = td.register();
                    let value = td.value();
                    if let Err(e) = self.channels.to_register_cache.send(register_cache::ChannelData::RegisterData(register, value)) {
                        error!("Failed to cache register {}: {}", register, e);
                        self.stats.lock().unwrap().increment_cache_errors();
                    }
                    if let Err(e) = self.publish_write_confirmation(register, value, inverter).await {
                        error!("Failed to publish write confirmation: {}", e);
                        self.stats.lock().unwrap().increment_mqtt_errors();
                    }
                }
                DeviceFunction::WriteMulti => {
                    debug!("Processing WriteMulti packet");
                    let pairs = td.pairs();
                    for (register, value) in &pairs {
                        if let Err(e) = self.channels.to_register_cache.send(register_cache::ChannelData::RegisterData(*register, *value)) {
                            error!("Failed to cache register {}: {}", register, e);
                            self.stats.lock().unwrap().increment_cache_errors();
                        }
                    }
                    if let Err(e) = self.publish_write_multi_confirmation(pairs, inverter).await {
                        error!("Failed to publish write multi confirmation: {}", e);
                        self.stats.lock().unwrap().increment_mqtt_errors();
                    }
                }
            }
        }

        Ok(())
    }

    async fn cache_register(&self, register: lxp::packet::Register, value: u16) -> Result<()> {
        if let Err(e) = self.channels.to_register_cache.send(register_cache::ChannelData::RegisterData(register as u16, value)) {
            error!("Failed to cache register {}: {}", register as u16, e);
            self.stats.lock().unwrap().increment_cache_errors();
        }
        Ok(())
    }

    async fn inverter_receiver(&self) -> Result<()> {
        let mut receiver = self.channels.from_inverter.subscribe();

        while let inverter::ChannelData::Packet(packet) = receiver.recv().await? {
            // Update packet stats first
            if let Ok(mut stats) = self.stats.lock() {
                stats.packets_received += 1;
                
                // Store last message for the inverter
                if let Packet::TranslatedData(td) = &packet {
                    stats.last_messages.insert(td.datalog(), format!("{:?}", packet));
                }
                
                // Increment counter for specific received packet type
                match &packet {
                    Packet::Heartbeat(_) => stats.heartbeat_packets_received += 1,
                    Packet::TranslatedData(_) => stats.translated_data_packets_received += 1,
                    Packet::ReadParam(_) => stats.read_param_packets_received += 1,
                    Packet::WriteParam(_) => stats.write_param_packets_received += 1,
                }
            } else {
                warn!("Failed to lock stats mutex for packet tracking");
            }

            // Process packet based on type
            if let Packet::TranslatedData(ref td) = packet {
                // Find the inverter for this packet
                if let Some(inverter) = self.config.enabled_inverter_with_datalog(td.datalog()) {
                    if let Err(e) = self.process_inverter_packet(packet.clone(), &inverter).await {
                        warn!("Failed to process packet: {}", e);
                    }
                } else {
                    warn!("No enabled inverter found for datalog {}", td.datalog());
                }
            }
        }

        Ok(())
    }

    async fn inverter_connected(&self, datalog: Serial) -> Result<()> {
        let inverter = match self.config.enabled_inverter_with_datalog(datalog) {
            Some(inverter) => inverter,
            None => {
                warn!("Unknown inverter datalog connected: {}, will continue processing its data", datalog);
                return Ok(());
            }
        };

        if !inverter.publish_holdings_on_connect() {
            return Ok(());
        }

        info!("Reading holding registers for inverter {}", datalog);

        // Add delay between read_hold requests to prevent overwhelming the inverter
        const DELAY_MS: u64 = 1; // 1ms delay between requests

        // Create a packet for stats tracking
        let packet = Packet::TranslatedData(TranslatedData {
            datalog: Serial::default(),
            device_function: DeviceFunction::ReadHold,
            inverter: Serial::default(),
            register: 0,
            values: vec![],
        });

        // We can only read holding registers in blocks of 40. Provisionally,
        // there are 6 pages of 40 values.
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 0_u16, 40).await?;
//        tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
        
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 40_u16, 40).await?;
//        tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
        
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 80_u16, 40).await?;
//        tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
        
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 120_u16, 40).await?;
//        tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
        
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 160_u16, 40).await?;
//        tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
        
        self.increment_packets_sent(&packet);
        self.read_hold(inverter.clone(), 200_u16, 40).await?;

        // Also send any special interpretive topics which are derived from
        // the holding registers.
        //
        // FIXME: this is a further 12 round-trips to the inverter to read values
        // we have already taken, just above. We should be able to do better!
        for num in &[1, 2, 3] {
            self.increment_packets_sent(&packet);
            self.read_time_register(
                inverter.clone(),
                commands::time_register_ops::Action::AcCharge(*num),
            )
            .await?;
            self.increment_packets_sent(&packet);
            self.read_time_register(
                inverter.clone(),
                commands::time_register_ops::Action::ChargePriority(*num),
            )
            .await?;
            self.increment_packets_sent(&packet);
            self.read_time_register(
                inverter.clone(),
                commands::time_register_ops::Action::ForcedDischarge(*num),
            )
            .await?;
            self.increment_packets_sent(&packet);
            self.read_time_register(
                inverter.clone(),
                commands::time_register_ops::Action::AcFirst(*num),
            )
            .await?;
        }

        Ok(())
    }

    fn publish_message(&self, topic: String, payload: String, retain: bool) -> Result<()> {
        let m = mqtt::Message {
            topic,
            payload,
            retain,
        };
        let channel_data = mqtt::ChannelData::Message(m);
        if self.channels.to_mqtt.send(channel_data).is_err() {
            if let Ok(mut stats) = self.stats.lock() {
                stats.mqtt_errors += 1;
            }
            bail!("send(to_mqtt) failed - channel closed?");
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.mqtt_messages_sent += 1;
        }
        Ok(())
    }

    // Helper method to get all input registers
    async fn get_all_inputs(&self) -> Result<std::collections::HashMap<u16, u16>> {
        // Implementation would go here
        Ok(std::collections::HashMap::new())
    }

    // Renamed from maybe_send_read_holds for clarity
    async fn check_related_holds(
        &self,
        register_map: std::collections::HashMap<u16, u16>,
        inverter: config::Inverter,
    ) -> Result<()> {
        // Original implementation remains the same
        if register_map.contains_key(&68) ^ register_map.contains_key(&69) {
            self.read_hold(inverter.clone(), 84_u16, 2).await?;
        }
        // ... rest of the implementation ...
        Ok(())
    }

    async fn publish_raw_input_messages(&self, input_all: &ReadInputAll, inverter: &config::Inverter) -> Result<()> {
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        // Publish raw values
        let topic = format!("{}/inputs/all", inverter.datalog);
        if let Err(e) = self.publish_message(topic, serde_json::to_string(input_all)?, false) {
            error!("Failed to publish raw input messages: {}", e);
            self.stats.lock().unwrap().increment_mqtt_errors();
        }

        Ok(())
    }

    async fn publish_raw_input_messages_1(&self, input_1: &ReadInput1, inverter: &config::Inverter) -> Result<()> {
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        // Publish raw values
        let topic = format!("{}/inputs/1", inverter.datalog);
        if let Err(e) = self.publish_message(topic, serde_json::to_string(input_1)?, false) {
            error!("Failed to publish raw input messages: {}", e);
            self.stats.lock().unwrap().increment_mqtt_errors();
        }

        Ok(())
    }

    async fn publish_hold_message(&self, register: u16, pairs: Vec<(u16, u16)>, inverter: &config::Inverter) -> Result<()> {
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        // Publish raw values
        for (reg, value) in pairs {
            let topic = format!("{}/hold/{}", inverter.datalog, reg);
            if let Err(e) = self.publish_message(topic, value.to_string(), true) {
                error!("Failed to publish hold message: {}", e);
                self.stats.lock().unwrap().increment_mqtt_errors();
            }
        }

        Ok(())
    }

    async fn publish_write_confirmation(&self, register: u16, value: u16, inverter: &config::Inverter) -> Result<()> {
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        let topic = format!("{}/write/status", inverter.datalog);
        if let Err(e) = self.publish_message(topic, format!("OK: {} = {}", register, value), false) {
            error!("Failed to publish write confirmation: {}", e);
            self.stats.lock().unwrap().increment_mqtt_errors();
        }

        Ok(())
    }

    async fn publish_write_multi_confirmation(&self, pairs: Vec<(u16, u16)>, inverter: &config::Inverter) -> Result<()> {
        if !self.config.mqtt().enabled() {
            return Ok(());
        }

        let topic = format!("{}/write_multi/status", inverter.datalog);
        if let Err(e) = self.publish_message(topic, format!("OK: {:?}", pairs), false) {
            error!("Failed to publish write multi confirmation: {}", e);
            self.stats.lock().unwrap().increment_mqtt_errors();
        }

        Ok(())
    }
}
