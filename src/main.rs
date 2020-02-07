use std::io::{Cursor, Read};
use std::thread;
use std::sync::mpsc;
use std::{env, fs, path::Path};

use byteorder::{ReadBytesExt, WriteBytesExt, NativeEndian};
use arr_macro::arr;

use sample::{Frame, Signal};
use sample::interpolate::{Converter, Floor};

use cpal::{StreamData, UnknownTypeOutputBuffer};
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};

use crate::samples::{Sample, SampleCursor};
use crate::patterns::Pattern;
use crate::notes::Note;
use crate::channel_state::ChannelState;

mod samples;
mod patterns;
mod notes;
mod channel_state;

static mut SAMPLE_RATE_CALC_VALUE: f64 = 7159090.5;

fn sample_rate(period: u16) -> f64 {
    let value = unsafe { SAMPLE_RATE_CALC_VALUE };
    value / (period as f64 * 2.0)
}

fn main() {
    let mut args = env::args();
    args.next();
    let file = args.next().expect("Provide a mod file as an argument");
    let file = Path::new(&file);
    let mod_data = fs::read(file).expect("Unable to read mod file");

    match env::var("PAL_MODE") {
        Ok(pal) => {
            if pal != "0" && pal != "false" {
                unsafe { SAMPLE_RATE_CALC_VALUE = 7093789.2; }
            }
        },
        Err(_) => ()
    };

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

    if file_tag != "M.K." && file_tag != "M!K!" && file_tag != "FLT4" && file_tag != "4CHN" {
        eprintln!("\nFile has file tag {}, can only read M.K., M!K! or FLT4 files", file_tag);
        return;
    }

    eprintln!("Song name: {}", song_name);
    eprintln!("Samples: {:#?}", samples);
    eprintln!("Pattern count: {}, Song End Jump Position: {}", number_of_patterns, song_end_jump);
    eprintln!("Patterns in file: {}", nop_in_file);
    eprintln!();
    
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
    let (tx, rx) = mpsc::sync_channel::<((u8, u8), <SampleCursor as Signal>::Frame)>(44100 / 50);

    let audio_thread = {
        if atty::is(atty::Stream::Stdout) {
            thread::spawn(move || {
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
                let _stream_id = event_loop.build_output_stream(&device, &format).unwrap();

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
                                            eprintln!("Playing Pattern {:02X} (index {:02X}), Line {:02X}",
                                                pattern_table[current_pattern as usize], current_pattern, current_line);
                                        }

                                        *elem = data.1[0];
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
            })
        } else {
            thread::spawn(move || {
                let mut current_pattern = 0;
                let mut current_line = 0;
                let mut stdout = std::io::stdout();
                while let Ok(data) = rx.recv() {
                    if (data.0).0 != current_pattern || (data.0).1 != current_line {
                        current_pattern = (data.0).0;
                        current_line = (data.0).1;
                        eprintln!("Playing Pattern {:02X} (index {:02X}), Line {:02X}",
                            pattern_table[current_pattern as usize], current_pattern, current_line);
                    }
                    stdout.write_f32::<NativeEndian>(data.1[0]).unwrap();
                }
            })
        }
    };

    let mut current_speed = 6;
    let mut current_tick = 0;
    let mut current_line = 0;
    let mut current_pattern = 0;
    let mut processed_line = false;

    let mut next_line = current_line;
    let mut next_pattern = current_pattern;

    let mut glissando = false;
    let mut channel_state = [ChannelState::new(); 4];
    let mut interpolators: [Option<Converter<SampleCursor, Floor<<SampleCursor as Signal>::Frame>>>; 4] = arr![None; 4];
    //let mut interpolators: [Option<Converter<SampleCursor, Sinc<[<SampleCursor as Signal>::Frame; 8]>>>; 4] = arr![None; 4];
    
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
                
                channel_state[i].arpeggio = (0, 0);
                channel_state[i].portamento = 0;

                channel_state[i].restart_sample_every = 0;
                channel_state[i].cut_sample_after = 0;

                match effect.number() {
                    0x0 => { // Arpeggio
                        if effect.arg_1() != 0 || effect.arg_1() != 0 {
                            println!("Arpeggio on channel {}", i);
                            channel_state[i].arpeggio = (effect.arg_1(), effect.arg_2());
                        }
                    }
                    0x1 => channel_state[i].portamento = effect.arg_joined() as i8, // Portamento up,
                    0x2 => channel_state[i].portamento = -(effect.arg_joined() as i8), // Portamento down
                    0x3 => {
                        match Note::from(channel.period()) {
                            Some(note) => channel_state[i].slide_to_note = Some((note, effect.arg_joined())),
                            None => ()
                        }
                    },
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
                        if atty::is(atty::Stream::Stdout) { // We don't want to infinitely pump to stdout
                            next_pattern = effect.arg_joined() as usize;
                            next_line = 0;
                            current_tick = 0;
                        }
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
                    0xe => match effect.arg_1() { // Extended effects
                        0x3 => { // Glissando (half a note slides)
                            if effect.arg_2() != 0 { glissando = true; }
                            else { glissando = false; }
                        },
                        0x5 => { // Set finetune
                            match &interpolators[i] {
                                Some(interpolator) => interpolator.source().sample().set_finetune(effect.arg_2() as i8), 
                                None => ()
                            }
                        },
                        0xc => {
                            if effect.arg_2() == 0 { interpolators[i] = None }
                            else { channel_state[i].cut_sample_after = effect.arg_2() }
                        }
                        _ => ()
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
                    let sample = &samples[channel.number() as usize - 1];
                    channel_state[i].period = channel.period();
                    channel_state[i].original_period = channel.period();
                    channel_state[i].finetune = sample.finetune();

                    let mut cursor = SampleCursor::from(sample);
                    //let interpolator = Linear::from_source(&mut cursor);
                    let period = match Note::from(channel.period()) {
                        Some(note) => note.get_period(sample.finetune()),
                        None => channel.period()
                    };

                    //let buf = ring_buffer::Fixed::from(arr![cursor.next(); 8]);
                    //let interpolator = Sinc::new(buf);
                    let interpolator = Floor::from_source(&mut cursor);
                    interpolators[i] = Some(Converter::from_hz_to_hz(
                        cursor, interpolator,
                        sample_rate(period), 44100.0
                    ));
                }
            }
        }

        for i in 0..4 {
            use std::io::{Seek, SeekFrom};
            if channel_state[i].arpeggio != (0, 0) {
                match Note::from(channel_state[i].original_period) {
                    Some(note) => {
                        let new_note = match current_tick % 3 {
                            0 => note,
                            1 => note.increment_half(channel_state[i].arpeggio.0),
                            2 => note.increment_half(channel_state[i].arpeggio.1),
                            _ => note,
                        };

                        let period = new_note.get_period(channel_state[i].finetune);
                        channel_state[i].period = new_note.get_period(0);
                        match &mut interpolators[i] {
                            Some(interpolator) => interpolator.set_hz_to_hz(sample_rate(period), 44100.0),
                            None => ()
                        }
                    },
                    None => ()
                }
            }

            if channel_state[i].portamento != 0 {
                match Note::from(channel_state[i].period) {
                    Some(note) => {
                        let new_note = {
                            if glissando == false {
                                if channel_state[i].portamento > 0 { note.increment(channel_state[i].portamento as u8) }
                                else { note.decrement((-channel_state[i].portamento) as u8) }
                            } else {
                                if channel_state[i].portamento > 0 { note.increment_half(channel_state[i].portamento as u8) }
                                else { note.decrement_half((-channel_state[i].portamento) as u8) }
                            }
                        };

                        let period = new_note.get_period(channel_state[i].finetune);
                        channel_state[i].period = new_note.get_period(0);
                        match &mut interpolators[i] {
                            Some(interpolator) => interpolator.set_hz_to_hz(sample_rate(period), 44100.0),
                            None => ()
                        }
                    },
                    None => ()
                };
            }

            if channel_state[i].restart_sample_every != 0 && i % channel_state[i].restart_sample_every as usize == 0 {
                match &mut interpolators[i] {
                    Some(interpolator) => interpolator.source_mut().seek(SeekFrom::Start(0)).unwrap(),
                    None => 0
                };
            }

            if channel_state[i].cut_sample_after != 0 && i == channel_state[i].cut_sample_after as usize {
                interpolators[i] = None;
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
            let mut combined: [f32; 1] = frames[0][j].mul_amp([0.25]);
            for i in 1..frames.len() {
                combined = combined.add_amp(frames[i][j].mul_amp([0.25]));
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
    eprintln!("\rDone converting                     \n");
    std::mem::drop(tx);
    audio_thread.join().unwrap();
}
