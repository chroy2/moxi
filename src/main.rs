#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use panic_probe as _;

use crate::sense::sense_task;

mod sense;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_nrf::init(Default::default());
    spawner.must_spawn(sense_task(p.TWISPI0, p.P1_00, p.P0_26));
}
