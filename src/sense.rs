use defmt::info;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    watch::{DynReceiver, Watch},
};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use microbit_bsp::embassy_nrf::{
    Peri, bind_interrupts,
    peripherals::{P0_26, P1_00, TWISPI0},
    twim::{self, Twim},
};

#[derive(Clone, Copy)]
pub struct SensorReadings {
    pub co2: u16,
    pub temperature: i8,
    pub humidity: u8,
}

const CO2_CONSUMERS: usize = 2;
static CO2: Watch<ThreadModeRawMutex, SensorReadings, CO2_CONSUMERS> = Watch::new();

pub fn get_receiver() -> Option<DynReceiver<'static, SensorReadings>> {
    CO2.dyn_receiver()
}

#[embassy_executor::task]
pub async fn sense_task(
    twi: Peri<'static, TWISPI0>,
    sda: Peri<'static, P1_00>,
    scl: Peri<'static, P0_26>,
) {
    bind_interrupts!(struct Irqs {
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c = Twim::new(
        twi,
        Irqs,
        sda,
        scl,
        Default::default(),
        &mut [], // empty ram buf
    );

    let mut scd = Scd4x::new(i2c, Delay);
    Timer::after_millis(30).await;

    _ = scd.stop_periodic_measurement().await;

    info!("Sensor serial number: {:?}", scd.serial_number().await);
    if let Err(e) = scd.start_periodic_measurement().await {
        defmt::panic!("Failed to start periodic measurement: {:?}", e);
    }

    let tx = CO2.sender();

    loop {
        if scd.data_ready().await.unwrap() {
            let m = scd.read_measurement().await.unwrap();
            info!(
                "CO2: {} Humidity: {} Temperature: {}",
                m.co2 as u16, m.humidity as u16, m.temperature as u16
            );
            tx.send(SensorReadings {
                co2: m.co2 as u16,
                humidity: m.humidity as u8,
                temperature: m.temperature as i8,
            });
        }

        Timer::after_millis(1000).await;
    }
}
