#![allow(unused)]

pub mod services;

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::select;
use microbit_bsp::ble::{MultiprotocolServiceLayer, SoftdeviceController};
use static_cell::StaticCell;
use trouble_host::prelude::*;

use crate::{
    ble::services::{
        battery::BatteryService,
        configuration::{BLE_NAME, ThingyConfigurationService},
        environment::{TesGas, TesTemperature, ThingyEnvironmentService},
        motion::{ThingyMotionService, TmsGravity},
        sound::ThingySoundService,
        ui::ThingyUiService,
    },
    sense,
};

#[gatt_server]
struct Server {
    config: ThingyConfigurationService,
    env: ThingyEnvironmentService,
    ui: ThingyUiService,
    sound: ThingySoundService,
    motion: ThingyMotionService,
    battery: BatteryService,
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) {
    mpsl.run().await
}

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 2;
type BleHostResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;

#[embassy_executor::task]
async fn host_task(mut runner: Runner<'static, SoftdeviceController<'static>, DefaultPacketPool>) {
    runner.run().await.unwrap()
}

pub async fn run(
    sdc: SoftdeviceController<'static>,
    mpsl: &'static MultiprotocolServiceLayer<'static>,
    spawner: Spawner,
) {
    spawner.must_spawn(mpsl_task(mpsl));

    // static random address: client will remember...
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    let resources = {
        static RESOURCES: StaticCell<BleHostResources> = StaticCell::new();
        RESOURCES.init(BleHostResources::new())
    };
    let stack = {
        static STACK: StaticCell<Stack<'_, SoftdeviceController<'_>, DefaultPacketPool>> =
            StaticCell::new();
        STACK.init(trouble_host::new(sdc, resources).set_random_address(address))
    };
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();
    spawner.must_spawn(host_task(runner));

    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: BLE_NAME,
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .expect("Failed to create GATT server");

    loop {
        match advertise(&mut peripheral, &server).await {
            Ok(conn) => {
                select(gatt_events(&conn), env_notifier(&conn, &server)).await;
            }
            Err(e) => warn!("[adv] {:?}", e),
        }
    }
}

async fn env_notifier(conn: &GattConnection<'_, '_, DefaultPacketPool>, server: &Server<'_>) {
    let gas = &server.env.gas;
    let temperature = &server.env.temperature;
    let humidity = &server.env.humidity;

    let mut rx = sense::get_receiver().unwrap();
    loop {
        let readings = rx.changed().await;
        let co2 = readings.co2;
        if let Err(e) = gas.notify(conn, &TesGas::new(co2)).await {
            warn!("[gatt] notification error: {}", e);
        }
        let temperature_reading = readings.temperature;
        if let Err(e) = temperature
            .notify(conn, &TesTemperature::new(temperature_reading))
            .await
        {
            warn!("[gatt] notification error: {}", e);
        }
        let humidity_reading = readings.humidity;
        if let Err(e) = humidity.notify(conn, &humidity_reading).await {
            warn!("[gatt] notification error: {}", e);
        }
    }
}

async fn gatt_events(conn: &GattConnection<'_, '_, DefaultPacketPool>) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => match event.accept() {
                Ok(reply) => reply.send().await,
                Err(e) => warn!("[gatt] error processing request: {:?}", e),
            },
            _ => (),
        }
    }
}

async fn advertise<'a, 'b, C: Controller>(
    peripheral: &mut Peripheral<'a, C, DefaultPacketPool>,
    server: &'b Server<'_>,
) -> Result<GattConnection<'a, 'b, DefaultPacketPool>, BleHostError<C::Error>> {
    const GAP_ADV_LIMIT: usize = 31;
    let mut ad_data = [0u8; GAP_ADV_LIMIT];
    let ad_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids128(&[services::configuration::TCS.into()]),
            AdStructure::CompleteLocalName(BLE_NAME.as_bytes()),
        ],
        &mut ad_data[..],
    )?;
    let mut sr_data = [0u8; GAP_ADV_LIMIT];
    let sr_len = AdStructure::encode_slice(
        &[AdStructure::ManufacturerSpecificData {
            company_identifier: services::configuration::MSP_NORDIC_COMPANY_ID,
            payload: &services::configuration::MSP_PAYLOAD,
        }],
        &mut sr_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &ad_data[0..ad_len],
                scan_data: &sr_data[0..sr_len],
            },
        )
        .await?;
    info!("[adv] Advertising; waiting for connection...");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] Connection established");
    Ok(conn)
}
