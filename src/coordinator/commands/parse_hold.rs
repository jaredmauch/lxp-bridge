/// Parse and decode a hold register value according to Table 8 of the protocol specification
pub fn parse_hold_register(reg: u16, value: u16) -> String {
    match reg {
        // System Information (0-24)
        0 => {
            let lithium_type = (value >> 12) & 0xF;
            let power_rating = (value >> 8) & 0xF;
            let lead_acid_type = (value >> 4) & 0xF;
            let battery_type = value & 0xF;
            format!("Model Info: {:#06x}\n  Lithium Type: {}\n  Power Rating: {}\n  Lead Acid Type: {}\n  Battery Type: {}", 
                value, lithium_type, power_rating, lead_acid_type, battery_type)
        }
        2..=6 => {
            // Serial number format: AB12345678
            // SN[0]=Year (A-Z), SN[1]=Week (0-9,A-Z), SN[2]=Week (0-9,A-Z)
            // SN[3]=Factory (0-9,A-Z), SN[4-6]=Product code (0-9,A-Z)
            // SN[7-9]=Batch number (0-9,A-Z)
            let part = reg - 1;
            format!("Serial Number Part {} ({}): {:#06x}", 
                part,
                match part {
                    1 => "Year",
                    2 => "Week",
                    3 => "Factory",
                    4..=6 => "Product Code",
                    7..=9 => "Batch Number",
                    _ => "Unknown"
                },
                value)
        }
        7 => format!("Hold Register: {} - Firmware Version Code: {}", reg, value),
        8 => format!("Hold Register: {} - Backup Firmware Version Code: {}", reg, value),
        9 => format!("Hold Register: {} - Slave CPU Version (Redundant): {:#06x}", reg, value),
        10 => format!("Hold Register: {} - Control CPU Version: {:#06x}", reg, value),
        11 => {
            let mut settings = Vec::new();
            if value & (1 << 0) != 0 { settings.push("Energy Record Clear"); }
            if value & (1 << 1) != 0 { settings.push("Reset All to Default"); }
            if value & (1 << 2) != 0 { settings.push("Adjustment Ratio Clear"); }
            if value & (1 << 3) != 0 { settings.push("Fault Record Clear"); }
            if value & (1 << 4) != 0 { settings.push("Monitor Data Clear"); }
            if value & (1 << 5) != 0 { settings.push("BMS Charge Switch On"); }
            if value & (1 << 6) != 0 { settings.push("BMS Discharge Switch On"); }
            if value & (1 << 7) != 0 { settings.push("Inverter Reboot"); }
            if value & (1 << 8) != 0 { settings.push("Reserved"); }
            if value & (1 << 9) != 0 { settings.push("Reserved"); }
            if value & (1 << 10) != 0 { settings.push("Reserved"); }
            if value & (1 << 11) != 0 { settings.push("Reserved"); }
            if value & (1 << 12) != 0 { settings.push("Reserved"); }
            if value & (1 << 13) != 0 { settings.push("Reserved"); }
            if value & (1 << 14) != 0 { settings.push("Reserved"); }
            if value & (1 << 15) != 0 { settings.push("Reserved"); }
            format!("Hold Register: {} - Reset Settings: {:#018b}\nActive settings: {}", reg, value, settings.join(", "))
        }
        12 => {
            let month = value >> 8;
            let year = value & 0xFF;
            format!("Time: Month={} (1-12), Year=20{:02} (17-255)", month, year)
        }
        13 => {
            let hour = value >> 8;
            let day = value & 0xFF;
            format!("Time: Hour={} (0-23), Day={} (1-31)", hour, day)
        }
        14 => {
            let second = value >> 8;
            let minute = value & 0xFF;
            format!("Time: Second={} (0-59), Minute={} (0-59)", second, minute)
        }
        15 => format!("Hold Register: {} - Communication Address: {} (0-150)", reg, value),
        16 => format!("Hold Register: {} - Language: {} (1=English)", reg, value),
        19 => format!("Hold Register: {} - Version: {}", reg, value),
        20 => {
            let mode = match value {
                0 => "No PV",
                1 => "PV1 Connected",
                2 => "PV2 Connected",
                3 => "Two Parallel PV",
                4 => "Two Separate PV",
                5 => "PV1&3 Connected (12K Hybrid)",
                6 => "PV2&3 Connected (12K Hybrid)",
                7 => "PV1&2&3 Connected (12K Hybrid)",
                _ => "Unknown"
            };
            format!("Hold Register: {} - PV Input Mode: {} - {}", reg, value, mode)
        }
        21 => {
            let mut features = Vec::new();
            if value & (1 << 0) != 0 { features.push("EPS Mode"); }
            if value & (1 << 1) != 0 { features.push("Over Frequency Load Reduction"); }
            if value & (1 << 2) != 0 { features.push("DRMS"); }
            if value & (1 << 3) != 0 { features.push("Low Voltage Ride Through"); }
            if value & (1 << 4) != 0 { features.push("Anti-islanding"); }
            if value & (1 << 5) != 0 { features.push("Neutral Detection"); }
            if value & (1 << 6) != 0 { features.push("Grid-connected Power Soft Start"); }
            if value & (1 << 7) != 0 { features.push("AC Charge"); }
            if value & (1 << 8) != 0 { features.push("Off-grid Seamless Switching"); }
            if value & (1 << 9) != 0 { features.push("Power On (0=Standby)"); }
            if value & (1 << 10) != 0 { features.push("Forced Discharge"); }
            if value & (1 << 11) != 0 { features.push("Forced Charge"); }
            if value & (1 << 12) != 0 { features.push("ISO"); }
            if value & (1 << 13) != 0 { features.push("GFCI"); }
            if value & (1 << 14) != 0 { features.push("DCI"); }
            if value & (1 << 15) != 0 { features.push("Feed In Grid"); }
            format!("Hold Register: {} - Function Enable Flags: {:#018b}\nEnabled features: {}", reg, value, features.join(", "))
        }
        22 => format!("Hold Register: {} - Start PV Voltage: {:.1} V (90.0-500.0V)", reg, (value as f64) / 10.0),
        23 => format!("Hold Register: {} - Grid Connection Wait Time: {} seconds (30-600s)", reg, value),
        24 => format!("Hold Register: {} - Grid Reconnection Wait Time: {} seconds (0-900s)", reg, value),

        // Grid Connection Limits (25-28)
        25 => format!("Hold Register: {} - Grid Connect Low Voltage: {:.1} V", reg, (value as f64) / 10.0),
        26 => format!("Hold Register: {} - Grid Connect High Voltage: {:.1} V", reg, (value as f64) / 10.0),
        27 => format!("Hold Register: {} - Grid Connect Low Frequency: {:.2} Hz", reg, (value as f64) / 100.0),
        28 => format!("Hold Register: {} - Grid Connect High Frequency: {:.2} Hz", reg, (value as f64) / 100.0),

        // Grid Protection Settings (29-53)
        29..=53 => {
            let desc = match reg {
                29 => "Grid Voltage Level 1 Under-voltage Protection",
                30 => "Grid Voltage Level 1 Over-voltage Protection",
                31 => "Grid Voltage Level 1 Under-voltage Protection Time",
                32 => "Grid Voltage Level 1 Over-voltage Protection Time",
                33 => "Grid Voltage Level 2 Under-voltage Protection",
                34 => "Grid Voltage Level 2 Over-voltage Protection",
                35 => "Grid Voltage Level 2 Under-voltage Protection Time",
                36 => "Grid Voltage Level 2 Over-voltage Protection Time",
                37 => "Grid Voltage Level 3 Under-voltage Protection",
                38 => "Grid Voltage Level 3 Over-voltage Protection",
                39 => "Grid Voltage Level 3 Under-voltage Protection Time",
                40 => "Grid Voltage Level 3 Over-voltage Protection Time",
                41 => "Grid Voltage Moving Average Over-voltage Protection",
                42 => "Grid Frequency Level 1 Under-frequency Protection",
                43 => "Grid Frequency Level 1 Over-frequency Protection",
                44 => "Grid Frequency Level 1 Under-frequency Protection Time",
                45 => "Grid Frequency Level 1 Over-frequency Protection Time",
                46 => "Grid Frequency Level 2 Under-frequency Protection",
                47 => "Grid Frequency Level 2 Over-frequency Protection",
                48 => "Grid Frequency Level 2 Under-frequency Protection Time",
                49 => "Grid Frequency Level 2 Over-frequency Protection Time",
                50 => "Grid Frequency Level 3 Under-frequency Protection",
                51 => "Grid Frequency Level 3 Over-frequency Protection",
                52 => "Grid Frequency Level 3 Under-frequency Protection Time",
                53 => "Grid Frequency Level 3 Over-frequency Protection Time",
                _ => "Unknown Grid Protection Setting"
            };
            
            if reg % 2 == 0 && reg <= 41 {
                format!("Hold Register: {} - {}: {:.1} V", reg, desc, (value as f64) / 10.0)
            } else if reg % 2 == 0 && reg > 41 {
                format!("Hold Register: {} - {}: {:.2} Hz", reg, desc, (value as f64) / 100.0)
            } else {
                format!("Hold Register: {} - {}: {} ms", reg, desc, value)
            }
        }

        // Power Quality Control (54-63)
        54 => format!("Hold Register: {} - Maximum Q Percent for Q(V) Curve: {}%", reg, value),
        55 => format!("Hold Register: {} - Q(V) Lower Voltage Point 1 (V1L): {:.1} V", reg, (value as f64) / 10.0),
        56 => format!("Hold Register: {} - Q(V) Lower Voltage Point 2 (V2L): {:.1} V", reg, (value as f64) / 10.0),
        57 => format!("Hold Register: {} - Q(V) Upper Voltage Point 1 (V1H): {:.1} V", reg, (value as f64) / 10.0),
        58 => format!("Hold Register: {} - Q(V) Upper Voltage Point 2 (V2H): {:.1} V", reg, (value as f64) / 10.0),
        59 => format!("Hold Register: {} - Reactive Power Command Type: {}", reg, value),
        60 => format!("Hold Register: {} - Active Power Percent Command: {}%", reg, value),
        61 => format!("Hold Register: {} - Reactive Power Percent Command: {}%", reg, value),
        62 => format!("Hold Register: {} - Power Factor Command: {:.3}", reg, (value as f64) / 1000.0),
        63 => format!("Hold Register: {} - Power Soft Start Slope: {}", reg, value),

        // System Control (64-67)
        64 => format!("Hold Register: {} - System Charge Rate: {}%", reg, value),
        65 => format!("Hold Register: {} - System Discharge Rate: {}%", reg, value),
        66 => format!("Hold Register: {} - Grid Charge Power Rate: {}%", reg, value),
        67 => format!("Hold Register: {} - AC Charge SOC Limit: {}%", reg, value),
        68 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgStart_0 AC charging start time_hour setting: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        69 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgEndTime_0 AC charging end time_hour: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        70 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgStart_1 AC charging start time_hour setting: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        71 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgEndTime_1 AC charging end time_hour: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        72 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgStart_2 AC charging start time_hour setting: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        73 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACChgEndTime_2 AC charging end time_hour: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        // Charging Priority Settings (74-79)
        74 => format!("Hold Register: {} - ChgFirstPowerCMD - Charging Priority Percentage: {}%", reg, value),
        75 => format!("Hold Register: {} - ChgFirstSOCLimit - Charging Priority SOC Limit: {}%", reg, value),
        76 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstStart_0 - Charging Priority Start Time: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        77 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstEnd_0 - Charging Priority End Time: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        78 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstStart_1 - Charging Priority Start Time 1: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        79 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstEnd_1 - Charging Priority End Time 1: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        80 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstStart_2 - Charging Priority Start Time 2: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        81 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ChgFirstEnd_2 Charging Priority End Time 2: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },

        // System Type and Battery Settings (80-82)
        82 => format!("ForcedDischgPowerCMD - Forced discharge SOC limit setting: {} %", value),
        // Grid Settings (83-84)
        83 => {
            let voltage_level = match value {
                0 => "220V",
                1 => "380V",
                _ => "Unknown"
            };
            format!("Grid Voltage Level: {} - {}", value, voltage_level)
        }
        84 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ForcedDischgStart_0 - Forced discharge start time_hour setting: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },
        85 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ForcedDischgStart_0 - Forced discharge end time_hour setting: {:02}:{:02} (HH:MM)", reg, hour, minute)
        },


        86 => format!("PV2 Power Rating: {:.1} kW", (value as f64) / 10.0),

        // Inverter Settings (87-88)
        87 => format!("Inverter Power Rating: {:.1} kW", (value as f64) / 10.0),
        88 => format!("Inverter Efficiency: {:.1}%", (value as f64) / 10.0),

        // Battery Settings (89-90)
        89 => format!("Battery Nominal Voltage: {:.1} V", (value as f64) / 10.0),
        90 => format!("Battery Nominal Capacity: {:.1} kWh", (value as f64) / 10.0),

        // System Settings (91-92)
        91 => {
            let system_mode = match value {
                0 => "Normal",
                1 => "Backup",
                2 => "ECO",
                _ => "Unknown"
            };
            format!("Hold Register: {} - System Mode: {} - {}", reg, value, system_mode)
        }
        92 => {
            let priority = match value {
                0 => "Battery",
                1 => "Grid",
                2 => "PV",
                _ => "Unknown"
            };
            format!("System Priority: {} - {}", value, priority)
        },

        // Time Settings (93-94)
        93 => format!("Time Zone: UTC{}", if value > 0 { format!("+{}", value) } else { value.to_string() }),
        94 => {
            let dst = match value {
                0 => "Off",
                1 => "On",
                _ => "Unknown"
            };
            format!("Daylight Saving Time: {} - {}", value, dst)
        },

        // Communication Settings (95-96)
        95 => {
            let protocol = match value {
                0 => "Modbus",
                1 => "RS485",
                _ => "Unknown"
            };
            format!("Communication Protocol: {} - {}", value, protocol)
        }
        96 => {
            let baud_rate = match value {
                0 => "9600",
                1 => "19200",
                2 => "38400",
                _ => "Unknown"
            };
            format!("Communication Baud Rate: {} - {}", value, baud_rate)
        },

        // Alarm Settings (97-98)
        97 => {
            let alarm_enable = match value {
                0 => "Off",
                1 => "On",
                _ => "Unknown"
            };
            format!("Alarm Enable: {} - {}", value, alarm_enable)
        }
        98 => format!("Alarm Delay: {} seconds", value),

        // Maintenance Settings (99-100)
        99 => {
            let maintenance_mode = match value {
                0 => "Off",
                1 => "On",
                _ => "Unknown"
            };
            format!("Maintenance Mode: {} - {}", value, maintenance_mode)
        }
        100 => format!("Hold Register: {} Maintenance Time: {} minutes", reg, value),
        118 => format!("Hold Register: {} VbatStartDerating: {} V", reg, value),
        119 => format!("Hold Register: {} wCT_PowerOffset: {} W", reg, value),
  
        134 => format!("Hold Register: {} UVFDerateStartPoint: {} Hz", reg, value), // 0.01Hz
        135 => format!("Hold Register: {} UVFDerateEndPoint: {} Hz", reg, value), // 0.01Hz
        136 => format!("Hold Register: {} OVFDerateRatio: {} ", reg, value), // %Pm/Hz Underfrequency load shedding slope

        137 => format!("Hold Register: {} SpecLoadCompensate: {} W", reg, value), // Maximum compensation amount for a specific load
        138 => format!("Hold Register: {} ChargePowerPercentCMD: {}", reg, value), // 0.1% Charging power percentage setting
        139 => format!("Hold Register: {} DischgPowerPercentCMD: {}", reg, value), // 0.1% Discharge power percentage setting

        140 => format!("Hold Register: {} ACChgPowerCMD: {}", reg, value), // 0.1% ACChgPowerCMD
        141 => format!("Hold Register: {} ChgFirstPowerCMD: {}", reg, value), // 0.1% ChgFirstPowerCMD

        142 => format!("Hold Register: {} ForcedDischgPowerCMD: {}", reg, value), // 0.1% ForcedDischgPowerCMD
        143 => format!("Hold Register: {} ActivePowerPercentCMD: {}", reg, value), // 0.1% ActivePowerPercentCMD

        144 => format!("Hold Register: {} FloatChargeVolt: {} V", reg, value), // 0.1V
        145 => format!("Hold Register: {} OutputPrioConfig: {}", reg, value), // 0-bat first 1-PV first 2-AC first

        146 => format!("Hold Register: {} LineMode: {}", reg, value), // 0-APL (90-280V 20ms) 1- UPS (170-280V 10ms) 2- GEN (90-280V 20ms)

        147 => format!("Hold Register: {} Battery capacity: {} Ah", reg, value), // Ah
        148 => format!("Hold Register: {} Battery nominal Voltage: {} V", reg, value), // 0.1v units

        149 => format!("Hold Register: {} EqualizationVolt: {} ", reg, value), // EqualizationVolt
        150 => format!("Hold Register: {} EqualizationInterval: {} ", reg, value), // Days (0-365) Equalization interval
        151 => format!("Hold Register: {} EqualizationTime: {} ", reg, value), // hour (0-24) Equalization time

        // AC load start time_hour + minute setting
        152 => { 
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF; 
            format!("Hold Register: {} - ACFirstStartHour_0: {:02}:{:02} (HH:MM)", reg, hour, minute)

        }
        // AC load stop time_hour + minute setting
        153 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACFirstEndHour_0: {:02}:{:02} (HH:MM)", reg, hour, minute)

        }
        154 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACFirstStartHour_1: {:02}:{:02} (HH:MM)", reg, hour, minute)

        } 
        // AC load stop time_hour + minute setting
        155 => {
            let minute = (value >> 8) & 0xFF; 
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACFirstEndHour_1: {:02}:{:02} (HH:MM)", reg, hour, minute)

        }
        156 => {
            let minute = (value >> 8) & 0xFF;
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACFirstStartHour_2: {:02}:{:02} (HH:MM)", reg, hour, minute)

        } 
        // AC load stop time_hour + minute setting
        157 => {
            let minute = (value >> 8) & 0xFF; 
            let hour = value & 0xFF;
            format!("Hold Register: {} - ACFirstEndHour_2: {:02}:{:02} (HH:MM)", reg, hour, minute)

        }
        158 => format!("Hold Register: {} - ACChgStartVolt: {:.1} V", reg, (value as f64) / 10.0),
        159 => format!("Hold Register: {} - ACChgEndVolt: {:.1} V", reg, (value as f64) / 10.0),

        // AC Charge Settings (160-161)
        160 => format!("Hold Register: {} - AC Charge Start SOC: {}%", reg, value),
        161 => format!("Hold Register: {} - AC Charge End SOC: {}%", reg, value),

        // Battery Warning Settings (162-169)
        162 => format!("Hold Register: {} - Battery Warning Voltage: {:.1} V", reg, (value as f64) / 10.0),
        163 => format!("Hold Register: {} - Battery Warning Recovery Voltage: {:.1} V", reg, (value as f64) / 10.0),
        164 => format!("Hold Register: {} - Battery Warning SOC: {}%", reg, value),
        165 => format!("Hold Register: {} - Battery Warning Recovery SOC: {}%", reg, value),
        166 => format!("Hold Register: {} - Battery Low to Utility Voltage: {:.1} V", reg, (value as f64) / 10.0),
        167 => format!("Hold Register: {} - Battery Low to Utility SOC: {}%", reg, value),
        168 => format!("Hold Register: {} - AC Charge Battery Current: {:.1} A", reg, (value as f64) / 10.0),
        169 => format!("Hold Register: {} - On Grid EOD Voltage: {:.1} V", reg, (value as f64) / 10.0),

        // AutoTest Parameters (170-175)
        170 => format!("Hold Register: {} - AutoTest Command: {}", reg, value),
        171 => {
            let status = (value >> 0) & 0xF;
            let step = (value >> 4) & 0xF;
            let status_desc = match status {
                0 => "Waiting - Test not started",
                1 => "Testing - Test in progress",
                2 => "Test Failed - Last test failed",
                3 => "Voltage Test OK - Voltage tests passed",
                4 => "Frequency Test OK - Frequency tests passed",
                5 => "Test Passed - All tests completed successfully",
                _ => "Unknown status"
            };
            let step_desc = match step {
                1 => "V1L Test - Testing lower voltage limit 1",
                2 => "V1H Test - Testing upper voltage limit 1",
                3 => "F1L Test - Testing lower frequency limit 1",
                4 => "F1H Test - Testing upper frequency limit 1",
                5 => "V2L Test - Testing lower voltage limit 2",
                6 => "V2H Test - Testing upper voltage limit 2",
                7 => "F2L Test - Testing lower frequency limit 2",
                8 => "F2H Test - Testing upper frequency limit 2",
                _ => "No Test Active"
            };
            format!("AutoTest Status: {:#06x}\nStatus: {} - {}\nStep: {} - {}", 
                value, status, status_desc, step, step_desc)
        }
        172 => {
            let value_f = (value as f64) * if value & 0x8000 != 0 { -0.1 } else { 0.1 };
            format!("Hold Register: {} - AutoTest Limit: {:.1} {}", reg, value_f,
                if (reg >= 171 && reg <= 172) || (reg >= 175 && reg <= 176) { "V" } else { "Hz" })
        }
        173 => format!("Hold Register: {} - AutoTest Default Time: {} ms", reg, value),
        174 => {
            let value_f = (value as f64) * if value & 0x8000 != 0 { -0.1 } else { 0.1 };
            format!("Hold Register: {} - AutoTest Trip Value: {:.1} {}", reg, value_f,
                if (reg >= 171 && reg <= 172) || (reg >= 175 && reg <= 176) { "V" } else { "Hz" })
        }
        175 => format!("Hold Register: {} - AutoTest Trip Time: {} ms", reg, value),

        180 => format!("Hold Register: {} - AFCIArcThreshold: {}", reg, value),
        181 => format!("Hold Register: {} - VoltWatt_V1: {}", reg, value), // 0.1v
        182 => format!("Hold Register: {} - VoltWatt_V2: {}", reg, value), // 0.1v

        183 => format!("Hold Register: {} - VoltWatt_DelayTime: {} ms", reg, value), // ms
        184 => format!("Hold Register: {} - VoltWatt_P2: {}", reg, value), // 0.1v

        185 => format!("Hold Register: {} - Vref_QV: {}", reg, value),
        186 => format!("Hold Register: {} - Vref_filtertime: {} seconds", reg, value), 

        187 => format!("Hold Register: {} - Q3_QV: {}", reg, value), 
        188 => format!("Hold Register: {} - Q4_QV: {}", reg, value),

        189 => format!("Hold Register: {} - P1_QP: {} %", reg, value),
        190 => format!("Hold Register: {} - P2_QP: {} %", reg, value),
        191 => format!("Hold Register: {} - P3_QP: {} %", reg, value),
        192 => format!("Hold Register: {} - P4_QP: {} %", reg, value),

        // Default case for unknown registers
        _ => format!("Unknown hold register {}: {}", reg, value),
    }
} 
