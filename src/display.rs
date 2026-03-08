use core::fmt::Write;
use embassy_time::{Duration, Timer};
use heapless::String;
use microbit_bsp::{
    display::{Bitmap, Frame, LedMatrix},
    embassy_nrf::gpio::Output,
};

use crate::sense;

const ROWS: usize = 5;
const COLS: usize = 5;

#[embassy_executor::task]
pub async fn display_task(mut matrix: LedMatrix<Output<'static>, ROWS, COLS>) {
    let mut rx = sense::get_receiver().unwrap();
    let mut txt: String<6> = String::new();
    loop {
        let readings = rx.get().await;
        let co2 = readings.co2;
        write!(&mut txt, " {}", co2).ok();
        matrix.scroll(txt.as_str()).await;
        txt.clear();
        Timer::after_millis(2000).await;

        let mut level = [Bitmap::empty(COLS); ROWS];
        for (i, row) in level.iter_mut().rev().enumerate() {
            if co2 as usize > (400 + 200 * i) {
                *row = Bitmap::new(0b0001_1111, COLS);
            } else {
                break;
            }
        }

        matrix
            .display(Frame::new(level), Duration::from_secs(4))
            .await;
    }
}
