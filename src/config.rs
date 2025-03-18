use crate::prelude::*;

use serde::Deserialize;
use serde_with::serde_as;
use serde_yaml;

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub inverters: Vec<Inverter>,
    pub mqtt: Mqtt,
    pub influx: Influx,
    #[serde(default = "Vec::new")]
    pub databases: Vec<Database>,

    pub scheduler: Option<Scheduler>,

    #[serde(default = "Config::default_loglevel")]
    pub loglevel: String,
}

// Inverter {{{
#[derive(Clone, Debug, Deserialize)]
pub struct Inverter {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    pub host: String,
    pub port: u16,
    #[serde(deserialize_with = "de_serial")]
    pub serial: Serial,
    #[serde(deserialize_with = "de_serial")]
    pub datalog: Serial,

    pub heartbeats: Option<bool>,
    pub publish_holdings_on_connect: Option<bool>,
    pub read_timeout: Option<u64>,
    pub use_tcp_nodelay: Option<bool>,
    pub register_block_size: Option<u16>,
    pub delay_ms: Option<u64>,
}
impl Inverter {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn serial(&self) -> Serial {
        self.serial
    }

    pub fn datalog(&self) -> Serial {
        self.datalog
    }

    pub fn heartbeats(&self) -> bool {
        self.heartbeats == Some(true)
    }

    pub fn publish_holdings_on_connect(&self) -> bool {
        self.publish_holdings_on_connect == Some(true)
    }

    pub fn read_timeout(&self) -> u64 {
        self.read_timeout.unwrap_or(900) // 15 minutes
    }

    pub fn use_tcp_nodelay(&self) -> bool {
        self.use_tcp_nodelay.unwrap_or(true)  // Default to true for backward compatibility
    }

    pub fn register_block_size(&self) -> u16 {
        self.register_block_size.unwrap_or(40)  // Default to 40 for backward compatibility
    }

    pub fn delay_ms(&self) -> u64 {
        self.delay_ms.unwrap_or(1000)  // Default to 1000ms if not specified
    }
} // }}}

// HomeAssistant {{{
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct HomeAssistant {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    #[serde(default = "Config::default_mqtt_homeassistant_prefix")]
    pub prefix: String,
}

impl HomeAssistant {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }
} // }}}

// Mqtt {{{
#[derive(Clone, Debug, Deserialize)]
pub struct Mqtt {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    pub host: String,
    #[serde(default = "Config::default_mqtt_port")]
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,

    #[serde(default = "Config::default_mqtt_namespace")]
    pub namespace: String,

    #[serde(default = "Config::default_mqtt_homeassistant")]
    pub homeassistant: HomeAssistant,

    pub publish_individual_input: Option<bool>,
}
impl Mqtt {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn username(&self) -> &Option<String> {
        &self.username
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn homeassistant(&self) -> &HomeAssistant {
        &self.homeassistant
    }

    pub fn publish_individual_input(&self) -> bool {
        self.publish_individual_input == Some(true)
    }
} // }}}

// Influx {{{
#[derive(Clone, Debug, Deserialize)]
pub struct Influx {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,

    pub database: String,
}
impl Influx {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn username(&self) -> &Option<String> {
        &self.username
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }

    pub fn database(&self) -> &str {
        &self.database
    }
} // }}}

// Database {{{
#[derive(Clone, Debug, Deserialize)]
pub struct Database {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    pub url: String,
}
impl Database {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn url(&self) -> &str {
        &self.url
    }
} // }}}

// Scheduler {{{
#[derive(Clone, Debug, Deserialize)]
pub struct Scheduler {
    #[serde(default = "Config::default_enabled")]
    pub enabled: bool,

    pub timesync_cron: Option<String>,
}
impl Scheduler {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn timesync_cron(&self) -> &Option<String> {
        &self.timesync_cron
    }
} // }}}

#[derive(Debug)]
pub struct ConfigWrapper {
    config: Rc<RefCell<Config>>,
}

impl Clone for ConfigWrapper {
    fn clone(&self) -> Self {
        Self {
            config: Rc::clone(&self.config),
        }
    }
}

impl ConfigWrapper {
    pub fn new(file: String) -> Result<Self> {
        let config = Rc::new(RefCell::new(Config::new(file)?));

        Ok(Self { config })
    }

    pub fn inverters(&self) -> Ref<Vec<Inverter>> {
        Ref::map(self.config.borrow(), |b| &b.inverters)
    }

    pub fn set_inverters(&self, new: Vec<Inverter>) {
        let mut c = self.config.borrow_mut();
        c.inverters = new;
    }

    pub fn enabled_inverters(&self) -> Vec<Inverter> {
        self.inverters()
            .iter()
            .filter(|inverter| inverter.enabled)
            .cloned()
            .collect()
    }

    pub fn inverter_with_host(&self, host: &str) -> Option<Inverter> {
        self.inverters()
            .iter()
            .find(|inverter| inverter.host == host)
            .cloned()
    }

    pub fn enabled_inverter_with_datalog(&self, datalog: Serial) -> Option<Inverter> {
        self.enabled_inverters()
            .iter()
            .find(|inverter| inverter.datalog == datalog)
            .cloned()
    }

    pub fn inverters_for_message(&self, message: &mqtt::Message) -> Result<Vec<Inverter>> {
        use mqtt::TargetInverter::*;

        let (target_inverter, _) = message.split_cmd_topic()?;
        let inverters = self.enabled_inverters();

        let r = match target_inverter {
            All => inverters,
            Serial(datalog) => inverters
                .iter()
                .filter(|i| i.datalog == datalog)
                .cloned()
                .collect(),
        };

        Ok(r)
    }

    pub fn mqtt(&self) -> Ref<Mqtt> {
        Ref::map(self.config.borrow(), |b| &b.mqtt)
    }

    pub fn influx(&self) -> Ref<Influx> {
        Ref::map(self.config.borrow(), |b| &b.influx)
    }

    pub fn influx_mut(&self) -> RefMut<Influx> {
        RefMut::map(self.config.borrow_mut(), |b: &mut Config| &mut b.influx)
    }

    pub fn databases(&self) -> Ref<Vec<Database>> {
        Ref::map(self.config.borrow(), |b| &b.databases)
    }

    pub fn set_databases(&self, new: Vec<Database>) {
        let mut c = self.config.borrow_mut();
        c.databases = new;
    }

    pub fn databases_mut(&self) -> RefMut<Vec<Database>> {
        RefMut::map(self.config.borrow_mut(), |b: &mut Config| &mut b.databases)
    }

    pub fn have_enabled_database(&self) -> bool {
        self.databases().iter().any(|database| database.enabled)
    }

    pub fn enabled_databases(&self) -> Vec<Database> {
        self.databases()
            .iter()
            .filter(|database| database.enabled)
            .cloned()
            .collect()
    }

    pub fn scheduler(&self) -> Ref<Option<Scheduler>> {
        Ref::map(self.config.borrow(), |b| &b.scheduler)
    }

    pub fn loglevel(&self) -> String {
        self.config.borrow().loglevel.to_owned()
    }
}

impl Config {
    pub fn new(file: String) -> Result<Self> {
        let content = std::fs::read_to_string(&file)
            .map_err(|err| anyhow!("error reading {}: {}", file, err))?;

        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        // Validate MQTT configuration
        if self.mqtt.enabled {
            if self.mqtt.port == 0 {
                bail!("mqtt.port must be between 1 and 65535");
            }
            if self.mqtt.host.is_empty() {
                return Err(anyhow!("MQTT host cannot be empty"));
            }
        }

        // Validate InfluxDB configuration
        if self.influx.enabled {
            if let Err(e) = url::Url::parse(&self.influx.url) {
                return Err(anyhow!("Invalid InfluxDB URL: {}", e));
            }
            if self.influx.database.is_empty() {
                return Err(anyhow!("InfluxDB database name cannot be empty"));
            }
        }

        // Validate database URLs
        for db in &self.databases {
            if db.enabled {
                if let Err(e) = url::Url::parse(db.url()) {
                    return Err(anyhow!("Invalid database URL: {}", e));
                }
            }
        }

        // Validate inverter configurations
        for (i, inv) in self.inverters.iter().enumerate() {
            if inv.enabled {
                if inv.port == 0 {
                    bail!("inverter[{}].port must be between 1 and 65535", i);
                }
                if inv.host.is_empty() {
                    return Err(anyhow!("Inverter host cannot be empty"));
                }
                if inv.read_timeout.unwrap_or(900) == 0 {
                    return Err(anyhow!("Invalid read timeout: 0"));
                }
            }
        }

        // Validate scheduler configuration
        if let Some(scheduler) = &self.scheduler {
            if scheduler.enabled {
                if let Some(cron) = &scheduler.timesync_cron {
                    if cron.is_empty() {
                        return Err(anyhow!("Scheduler cron expression cannot be empty"));
                    }
                }
            }
        }

        Ok(())
    }

    fn default_mqtt_port() -> u16 {
        1883
    }
    fn default_mqtt_namespace() -> String {
        "lxp".to_string()
    }

    fn default_mqtt_homeassistant() -> HomeAssistant {
        HomeAssistant {
            enabled: Self::default_enabled(),
            prefix: Self::default_mqtt_homeassistant_prefix(),
        }
    }

    fn default_mqtt_homeassistant_prefix() -> String {
        "homeassistant".to_string()
    }

    fn default_enabled() -> bool {
        true
    }

    fn default_loglevel() -> String {
        "debug".to_string()
    }
}

fn de_serial<'de, D>(deserializer: D) -> Result<Serial, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse().map_err(serde::de::Error::custom)
}
