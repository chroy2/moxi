#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use microbit_bsp::Microbit;
use panic_probe as _;

use crate::{display::display_task, sense::sense_task};

mod ble;
mod display;
mod sense;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let b = Microbit::new(Default::default()); //board support package

    spawner.must_spawn(sense_task(b.twispi0, b.p20, b.p19));
    spawner.must_spawn(display_task(b.display));
    let (sdc, mpsl) = b.ble.init(b.timer0, b.rng).unwrap();
    ble::run(sdc, mpsl, spawner).await;
}
