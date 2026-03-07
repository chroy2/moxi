#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use microbit_bsp::Microbit;
use panic_probe as _;

use crate::{display::display_task, sense::sense_task};

mod display;
mod sense;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = Microbit::new(Default::default()); //board support package

    spawner.must_spawn(sense_task(p.twispi0, p.p20, p.p19));
    spawner.must_spawn(display_task(p.display));
}
