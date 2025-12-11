use clap::{Arg, Command};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleRate, Stream, StreamConfig,
};
use scanner_rust::{ScannerAscii, ScannerError};
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use guitarhero::guitar::Guitar;
use rand::rng;
fn main() -> anyhow::Result<()> {
    let opts = Command::new("guitarhero")
        .version(env!("CARGO_PKG_VERSION"))
        .args(&[Arg::new("file").required(false).help("The file to play")])
        .get_matches();
    let dev = cpal::default_host().default_output_device().unwrap();
    let mut config = dev.default_output_config()?.config();
    config.sample_rate = SampleRate(44100);
    config.channels = 1;
    let mut rng = rng();
    let guitar = Arc::new(Mutex::new(Guitar::new(config.sample_rate.0)));
    let stream = build_guitar_stream(dev, guitar.clone(), config);
    stream.play()?;
    if let Some(path) = opts.get_one::<String>("file") {
        let mut sheet = ScannerAscii::new(File::open(path)?);
        loop {
            // next_isize() returns Ok(None) upon reaching EOF.
            if let Some(pitch) = sheet.next_isize().or::<ScannerError>(Ok(None))? {
                let duration = if let Some(d) = sheet.next_f64()? {
                    d
                } else {
                    0.0
                };
                guitar.lock().unwrap().pluck(pitch, &mut rng);
                if pitch != 37 || duration != 0.0 {
                    println!("{} {}", pitch, duration);
                }
                if duration > 0.0 {
                    std::thread::sleep(std::time::Duration::from_secs_f64(duration));
                }
            } else {
                break;
            }
        }
    } else {
        for to_pluck in -24..13 {
            guitar.lock().unwrap().pluck(to_pluck, &mut rng);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    Ok(std::thread::sleep(std::time::Duration::from_millis(2000)))
}

fn build_guitar_stream(dev: Device, guitar: Arc<Mutex<Guitar>>, config: StreamConfig) -> Stream {
    dev.build_output_stream(
        &config.clone(),
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // FIXME: no support for dual-channel output
            for frame in output.chunks_mut(config.channels as usize) {
                for sample in frame.iter_mut() {
                    *sample = guitar.lock().unwrap().tick();
                }
            }
        },
        |err| eprintln!("Error building output sound stream: {}", err),
        None,
    )
    .unwrap()
}
