use audioadapter_buffers::direct::InterleavedSlice;
use clap::{Arg, Command};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
use guitarhero::guitar::Guitar;
use rand::rng;
use rubato::{Async, FixedAsync, PolynomialDegree, Resampler};
use scanner_rust::{ScannerAscii, ScannerError};
use std::{
    collections::VecDeque,
    fs::File,
    sync::{Arc, Mutex},
};
fn main() -> anyhow::Result<()> {
    let opts = Command::new("guitarhero")
        .version(env!("CARGO_PKG_VERSION"))
        .args(&[Arg::new("file").required(false).help("The file to play")])
        .get_matches();
    let dev = cpal::default_host().default_output_device().unwrap();
    let mut config = dev.default_output_config()?.config();
    let sample_rate = config.sample_rate.0 as f64;
    match *dev.default_output_config()?.buffer_size() {
        cpal::SupportedBufferSize::Range { min, .. } => {
            config.buffer_size = cpal::BufferSize::Fixed(512.max(min))
        }
        cpal::SupportedBufferSize::Unknown => (),
    }
    let mut rng = rng();
    let guitar = Arc::new(Mutex::new(Guitar::new(44100)));
    let outbuf = Arc::new(Mutex::new(VecDeque::from(vec![0f32; 1024])));
    let resample_ratio = sample_rate / 44100f64;
    let mut resampler = Async::<f32>::new_poly(
        resample_ratio,
        1.1,
        PolynomialDegree::Septic,
        1024,
        1,
        FixedAsync::Output,
    )?;
    let stream = build_guitar_stream(dev, outbuf.clone(), config);
    stream.play()?;
    if let Some(path) = opts.get_one::<String>("file") {
        let mut sheet = ScannerAscii::new(File::open(path)?);

        let mut ibuf = vec![0f32; (10.0 * 44100f64) as usize];
        let mut obuf = vec![0f32; (10.0 * sample_rate) as usize * 2];
        loop {
            // next_isize() returns Ok(None) upon reaching EOF.
            if let Some(pitch) = sheet.next_isize().or::<ScannerError>(Ok(None))? {
                let duration = if let Some(d) = sheet.next_f64()? {
                    d
                } else {
                    0.0
                };
                guitar.lock().unwrap().pluck(pitch, &mut rng);
                if duration > 0.0 {
                    {
                        let sample_count = (duration * 44100f64) as usize;

                        // hopefully not getting here...
                        if sample_count > ibuf.len() {
                            ibuf.resize(sample_count, 0.0);
                            obuf.resize(
                                (sample_count as f64 * (resample_ratio + 1.0)) as usize,
                                0.0,
                            );
                        }
                        let obuf_len = obuf.len();
                        // fill pre-resample buffer
                        for i in 0..sample_count {
                            ibuf[i] = guitar.lock().unwrap().tick();
                        }
                        let input_adapter = InterleavedSlice::new(&ibuf, 1, ibuf.len()).unwrap();
                        let mut output_adapter =
                            InterleavedSlice::new_mut(&mut obuf, 1, obuf_len).unwrap();
                        let (_, nbr_out) = resampler.process_all_into_buffer(
                            &input_adapter,
                            &mut output_adapter,
                            sample_count,
                            None,
                        )?;
                        let mut already_out = 0usize;
                        while already_out < nbr_out {
                            let mut buf = outbuf.lock().unwrap();
                            for i in &obuf[already_out..nbr_out.min(already_out + 44100)] {
                                buf.push_front(*i);
                            }
                            drop(buf);
                            // std::thread::sleep(std::time::Duration::from_secs_f64(0.01));
                            already_out += 44100;
                        }
                        let waittime = if outbuf.lock().unwrap().len() > (sample_rate as usize) {
                            std::time::Duration::from_secs_f64(0.5)
                        } else {
                            std::time::Duration::from_secs_f64(0.01)
                        };
                        std::thread::sleep(waittime);
                    }
                }
            } else {
                break;
            }
        }
        let buf = outbuf.lock().unwrap();
        if buf.len() > 0 {
            let waittime = buf.len() as f64 / sample_rate;
            drop(buf);
            std::thread::sleep(std::time::Duration::from_secs_f64(waittime));
        }
    } else {
        for to_pluck in -24..13 {
            guitar.lock().unwrap().pluck(to_pluck, &mut rng);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    Ok(())
}

fn build_guitar_stream(
    dev: Device,
    resampled: Arc<Mutex<VecDeque<f32>>>,
    config: StreamConfig,
) -> Stream {
    dev.build_output_stream(
        &config.clone(),
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in output.chunks_mut(config.channels as usize) {
                let next = resampled.lock().unwrap().pop_back().unwrap_or_else(|| 0f32);
                for sample in frame.iter_mut() {
                    *sample = next;
                }
            }
        },
        |err| eprintln!("Error building output sound stream: {}", err),
        None,
    )
    .unwrap()
}
