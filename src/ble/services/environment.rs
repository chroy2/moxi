use trouble_host::prelude::*;

use crate::impl_fixedgattvalue;

use super::ThingyUuid;

pub const TES: ThingyUuid = ThingyUuid(0x0200);

pub const TES_TEMPERATURE: ThingyUuid = ThingyUuid(0x0201);
pub const TES_PRESSURE: ThingyUuid = ThingyUuid(0x0202);
pub const TES_HUMIDITY: ThingyUuid = ThingyUuid(0x0203);
pub const TES_GAS: ThingyUuid = ThingyUuid(0x0204);
pub const TES_COLOR: ThingyUuid = ThingyUuid(0x0205);
pub const TES_CONFIG: ThingyUuid = ThingyUuid(0x0206);

#[gatt_service(uuid = TES)]
pub struct ThingyEnvironmentService {
    #[characteristic(uuid = TES_TEMPERATURE, notify)]
    pub temperature: TesTemperature,
    #[characteristic(uuid = TES_PRESSURE, notify)]
    pub pressure: TesPressure,
    #[characteristic(uuid = TES_HUMIDITY, notify)]
    pub humidity: u8,
    #[characteristic(uuid = TES_GAS, notify)]
    pub gas: TesGas,
    #[characteristic(uuid = TES_COLOR, notify)]
    pub color: TesColor,
    #[characteristic(uuid = TES_CONFIG, read, write)]
    pub config: TesConfiguration,
}

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
pub struct TesTemperature {
    integer: i8,
    decimal: u8,
}

impl TesTemperature {
    pub const fn new(temperature_c: i8) -> Self {
        Self {
            integer: temperature_c,
            decimal: 0,
        }
    }
}

impl_fixedgattvalue!(TesTemperature);

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
pub struct TesPressure {
    pub integer: i32,
    decimal: u8,
}

impl TesPressure {
    pub const fn new(pressure_hpa: u16) -> Self {
        Self {
            integer: pressure_hpa as i32,
            decimal: 0,
        }
    }
}

impl_fixedgattvalue!(TesPressure);

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
pub struct TesGas {
    co2_ppm: u16,
    tvoc_ppb: u16,
}

impl TesGas {
    pub const fn new(co2_ppm: u16) -> Self {
        Self {
            co2_ppm,
            tvoc_ppb: 0,
        }
    }
}

impl_fixedgattvalue!(TesGas);

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
pub struct TesColor {
    red: u16,
    green: u16,
    blue: u16,
    clear: u16,
}

impl TesColor {
    pub const fn new(red: u16, green: u16, blue: u16, clear: u16) -> Self {
        Self {
            red,
            green,
            blue,
            clear,
        }
    }
}

impl_fixedgattvalue!(TesColor);

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TesConfiguration {
    temperature_interval_ms: u16,
    pressure_interval_ms: u16,
    humidity_interval_ms: u16,
    color_interval_ms: u16,
    gas_interval_mode: u8,
    color_config: [u8; 3],
}

impl Default for TesConfiguration {
    fn default() -> Self {
        Self {
            temperature_interval_ms: 2000,
            pressure_interval_ms: 2000,
            humidity_interval_ms: 2000,
            color_interval_ms: 1500,
            gas_interval_mode: 2,
            color_config: [107, 78, 29],
        }
    }
}

impl_fixedgattvalue!(TesConfiguration);
