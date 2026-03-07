use defmt::{info, warn};
use embassy_executor::Spawner;
use microbit_bsp::ble::{MultiprotocolServiceLayer, SoftdeviceController};
use static_cell::StaticCell;
use trouble_host::prelude::*;

#[gatt_server]
struct Server {
    battery_service: BatteryService,
}

#[gatt_service(uuid = service::BATTERY)]
struct BatteryService {
    #[characteristic(uuid= characteristic::BATTERY_LEVEL, read, notify)]
    level: u8,
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) {
    mpsl.run().await
}

const BLE_NAME: &str = "moxi";
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 2;

type BleHostResources = HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>;

#[embassy_executor::task]
async fn host_task(mut runner: Runner<'static, SoftdeviceController<'static>, DefaultPacketPool>) {
    runner.run().await.unwrap();
}

pub async fn run(
    sdc: SoftdeviceController<'static>,
    mpsl: &'static MultiprotocolServiceLayer<'static>,
    spawner: Spawner,
) {
    spawner.must_spawn(mpsl_task(mpsl));

    // [0xff, 0xa3, 0xa3, 0xa3, 0xa3, 0xff] static random address client will
    // remember
    let address: Address = Address::random([0xff, 0xa3, 0xa3, 0xa3, 0xa3, 0xff]);
    let resources = {
        static RESOURCES: StaticCell<BleHostResources> = StaticCell::new();
        RESOURCES.init(BleHostResources::new())
    };
    let stack = {
        static STACK: StaticCell<Stack<'_, SoftdeviceController<'static>, DefaultPacketPool>> =
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
            Ok(conn) => (), // ..
            Err(err) => warn!("[adv] {:?}", err),
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
            AdStructure::ServiceUuids16(&[service::BATTERY.to_le_bytes()]),
            AdStructure::CompleteLocalName(BLE_NAME.as_bytes()),
        ],
        &mut ad_data,
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &ad_data[0..ad_len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] Advertising; waiting for connection...");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    Ok(conn)
}
