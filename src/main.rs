use std::io::{Cursor, Read};
use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use std::{env, fs, path::Path};

use byteorder::ReadBytesExt;
use arr_macro::arr;

use sample::{Frame, Signal};
use sample::interpolate::{Converter, Linear};

use cpal::{StreamData, UnknownTypeOutputBuffer};
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};

use crate::samples::{Sample, SampleCursor};
use crate::patterns::Pattern;
use crate::channel_state::ChannelState;

mod samples;
mod patterns;
mod channel_state;

fn sample_rate(period: u16) -> f64 {
    7093789.2 / (period as f64 * 2.0)
}

fn main() {
    let mut args = env::args();
    args.next();
    let file = args.next().expect("Provide a mod file as an argument");
    let file = Path::new(&file);
    let mod_data = fs::read(file).expect("Unable to read mod file");

    let mut cursor = Cursor::new(mod_data.as_slice());
    let song_name = {
        let mut buf: [u8; 20] = [0; 20];
        cursor.read_exact(&mut buf).unwrap();
        
        let mut len = 20;
        for i in 0..len {
            if buf[i] == 0 {
                len = i;
                break;
            }
        }

        String::from_utf8_lossy(&buf[0..len]).into_owned()
    };

    let mut samples: [Sample; 31] = arr![Sample::from(&mut cursor).unwrap(); 31];
    let number_of_patterns = cursor.read_u8().unwrap();
    let song_end_jump = cursor.read_u8().unwrap();
    let mut pattern_table: [u8; 128] = [0; 128];
    cursor.read_exact(&mut pattern_table).unwrap();
    let file_tag = {
        let mut buf: [u8; 4] = [0; 4];
        cursor.read_exact(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    };

    let nop_in_file = pattern_table.iter().max().unwrap() + 1;

    if file_tag != "M.K." {
        println!("\nFile has file tag {}, can only read M.K.", file_tag);
        return;
    }

    println!("Song name: {}", song_name);
    println!("Samples: {:#?}", samples);
    println!("Pattern count: {}, Song End Jump Position: {}", number_of_patterns, song_end_jump);
    println!("Patterns in file: {}", nop_in_file);
    println!();
    
    let mut patterns = Vec::new();
    for _ in 0..nop_in_file {
        let mut buf = [0 as u8; std::mem::size_of::<Pattern>()];
        cursor.read_exact(&mut buf).unwrap();
        patterns.push(Pattern::from(&buf[..]));
    }

    for sample in &mut samples {
        if sample.length() > 0 {
            let mut buf = vec![0; sample.length() as usize];
            cursor.read_exact(&mut buf).unwrap();
            sample.set_data(buf);
        }
    }

    // 10 seconds of buffer
    let (tx, rx) = mpsc::sync_channel::<((u8, u8), <SampleCursor as Signal>::Frame)>(44100 * 10);

    let audio_thread = thread::spawn(move || {
        let host = cpal::default_host();
        let event_loop = host.event_loop();
        let device = host.default_output_device().expect("no output device");
        let mut supported_formats_range = device.supported_output_formats()
            .expect("error while querying formats");
        let mut format = supported_formats_range.find(|format| {
            if format.data_type != cpal::SampleFormat::F32 { return false; }
            if format.channels != 1 { return false; }
            if format.min_sample_rate.0 > 44100 || format.max_sample_rate.0 < 44100 { return false; }
            true
        }).expect("No suitable format").with_max_sample_rate();
        format.sample_rate = cpal::SampleRate(44100);
        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

        let mut current_pattern = 0;
        let mut current_line = 0;
        event_loop.run(move |stream_id, stream_result| {
            let stream_data = match stream_result {
                Ok(data) => data,
                Err(err) => {
                    eprintln!("An error occured on stream {:?}: {}", stream_id, err);
                    return;
                }
            };

            match stream_data {
                StreamData::Output { buffer: UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        match rx.try_recv() {
                            Ok(data) => {
                                if (data.0).0 != current_pattern || (data.0).1 != current_line {
                                    current_pattern = (data.0).0;
                                    current_line = (data.0).1;
                                    println!("Playing Pattern {}, Line {}", current_pattern, current_line);
                                }

                                let value = data.1[0];
                                *elem = value;
                            },
                            Err(error) if error == mpsc::TryRecvError::Disconnected => {
                                panic!("MPSC channel disconnected");
                            },
                            Err(_) => *elem = 0.0
                        }
                    }
                },
                _ => (),
            }
        });
    });

    let mut current_speed = 6;
    let mut current_tick = 0;
    let mut current_line = 0;
    let mut current_pattern = 0;
    let mut processed_line = false;

    let mut next_line = current_line;
    let mut next_pattern = current_pattern;

    let mut channel_state = [ChannelState::new(); 4];    
    let mut interpolators: [Option<Converter<SampleCursor, Linear<<SampleCursor as Signal>::Frame>>>; 4] = arr![None; 4];

    'main: loop {
        if next_pattern != current_pattern {
            current_pattern = next_pattern;
            processed_line = false;
        }
        if next_line != current_line {
            current_line = next_line;
            processed_line = false;
        }

        if !processed_line {
            let line = &patterns[pattern_table[current_pattern] as usize][current_line];
            for i in 0..4 {
                let channel = line[i];
                let effect = channel.effect();
                channel_state[i].volume = channel_state[i].volume + channel_state[i].volume_slide;
                if channel_state[i].volume > 64 { channel_state[i].volume = 64; channel_state[i].volume_slide = 0; }
                if channel_state[i].volume < 0 { channel_state[i].volume = 0; channel_state[i].volume_slide = 0; }
                match effect.number() {
                    0x5 => { // Continue Slide to Note, do Volume Slide
                        if effect.arg_1() != 0 { channel_state[i].volume_slide = effect.arg_1() as i8; }
                        else { channel_state[i].volume_slide = -(effect.arg_2() as i8); }
                    },
                    0x6 => { // Continue Vibrato, do Volume Slide
                        if effect.arg_1() != 0 { channel_state[i].volume_slide = effect.arg_1() as i8; }
                        else { channel_state[i].volume_slide = -(effect.arg_2() as i8); }
                    },
                    0xa => { // Volume Slide
                        if effect.arg_1() != 0 { channel_state[i].volume_slide = effect.arg_1() as i8; }
                        else { channel_state[i].volume_slide = -(effect.arg_2() as i8); }
                    },
                    0xb => { // Position Jump
                        next_pattern = effect.arg_joined() as usize;
                        next_line = 0;
                        current_tick = 0;
                    },
                    0xc => { // Set Volume
                        channel_state[i].volume_slide = 0;
                        if effect.arg_joined() > 64 { channel_state[i].volume = 64; }
                        else { channel_state[i].volume = effect.arg_joined() as i8; }
                    },
                    0xd => { // Pattern Break
                        next_pattern += 1;
                        next_line = ((effect.arg_1() * 10) + effect.arg_2()) as usize;
                        current_tick = 0;
                    },
                    0xf => { // Set Speed
                        channel_state[i].volume_slide = 0;
                        if effect.arg_joined() != 0 {
                            current_speed = effect.arg_joined();
                        }
                    }
                    _ => ()
                }

                if channel.number() != 0 && channel.period() != 0 {
                    channel_state[i].period = channel.period();
                    let mut cursor = SampleCursor::from(&samples[channel.number() as usize - 1]);
                    let interpolator = Linear::from_source(&mut cursor);
                    interpolators[i] = Some(Converter::from_hz_to_hz(
                        cursor, interpolator,
                        sample_rate(channel.period()), 44100.0
                    ));
                }
            }
        }

        current_tick += 1;

        let mut frames: Vec<Vec<<SampleCursor as Signal>::Frame>> = Vec::new();
        for (i, interpolator) in interpolators.iter_mut().enumerate() {
            match interpolator {
                Some(interpolator) => {
                    let mut buf = Vec::new();
                    for _ in 0..((44100.0 * 0.02) as usize) {
                        let sample = interpolator.source().sample();
                        let volume: [f32; 1] = [(channel_state[i].volume as f32) / 64.0];
                        let volume: [f32; 1] = volume.mul_amp([(sample.volume() as f32) / 64.0]);
                        buf.push(interpolator.next().mul_amp(volume));
                        if interpolator.is_exhausted() { panic!("bruh"); }
                    }
                    frames.push(buf);
                },
                None => {
                    let mut buf = Vec::new();
                    for _ in 0..((44100.0 * 0.02) as usize) {
                        buf.push([0.0]);
                    }
                    frames.push(buf);
                }
            }
        }

        for j in 0..frames[0].len() {
            let mut combined: [f32; 1] = frames[0][j];
            for i in 1..frames.len() {
                combined = combined.add_amp(frames[i][j]);
            }
            tx.send(((current_pattern as u8, current_line as u8), combined)).unwrap();
        }

        if current_tick >= current_speed {
            current_tick = 0;
            next_line += 1;
        }
        if next_line >= 64 {
            next_line = 0;
            next_pattern += 1;
        }
        if next_pattern >= number_of_patterns as usize { break 'main; }
    }
    println!("\rDone converting                     \n");
    std::mem::drop(tx);
    audio_thread.join().unwrap();
}
