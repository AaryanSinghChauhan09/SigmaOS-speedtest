//! SigmaOS component binary: speedtest

use sigmaos_speedtest::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-speedtest: ok");
    Ok(())
}
