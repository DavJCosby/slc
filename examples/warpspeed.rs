mod drivers;
use drivers::warpspeed;

mod resources;
use resources::tui::SledTerminalDisplay;

use spatial_led::{scheduler::Scheduler, Sled};
use palette::rgb::Rgb;

fn main() {
    let sled = Sled::<Rgb>::new("./examples/resources/complex_room.yap").unwrap();
    let mut display = SledTerminalDisplay::start("Warpspeed", sled.domain());
    let mut driver = warpspeed::build_driver();
    driver.mount(sled);

    let mut vector: Vec<Rgb> = vec![];
    vector.extend([
        Rgb::new(1.0, 0.0, 0.0),
        Rgb::new(0.0, 1.0, 0.0),
        Rgb::new(0.0, 0.0, 1.0),
    ]);

    let mut scheduler = Scheduler::new(500.0);
    scheduler.loop_until_err(|| {
        driver.step();
        display.set_leds(driver.colors_and_positions());
        display.refresh().unwrap();
        Ok(())
    });
}
