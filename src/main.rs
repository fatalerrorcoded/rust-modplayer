use std::io::{Cursor, Read};
use std::thread;
use std::time::Duration;

use byteorder::ReadBytesExt;
use arr_macro::arr;

use crate::samples::Sample;
use crate::patterns::Pattern;
use crate::channel_state::ChannelState;

mod samples;
mod patterns;
mod channel_state;

static MOD_DATA: &[u8] = include_bytes!("../epic.mod");

fn main() {
    let mut cursor = Cursor::new(MOD_DATA);
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
    for pattern in pattern_table.iter() {
        print!("{}, ", pattern);
    }
    println!();
    
    let mut patterns = Vec::new();
    for _ in 0..nop_in_file {
        let mut buf = [0 as u8; std::mem::size_of::<Pattern>()];
        cursor.read_exact(&mut buf).unwrap();
        patterns.push(Pattern::from(&buf[..]));
    }

    for line in 0..8 {
        println!("Pattern 0, Line {}: {:#?}", line, patterns[0][line]);
    }

    for sample in &mut samples {
        if sample.length() > 0 {
            let mut buf = vec![0; sample.length() as usize];
            cursor.read_exact(&mut buf).unwrap();
            sample.set_data(buf);
        }
    }

    let mut current_speed = 6;
    let mut current_tick = 0;
    let mut current_line = 0;
    let mut current_pattern = 0;
    let mut processed_line = false;

    let channel_state = [ChannelState::new(); 4];    

    'main: loop {
        if !processed_line {
            let line = &patterns[pattern_table[current_pattern] as usize][current_line];
            for i in 0..4 {
                let effect = line[i].effect();
                match effect.number() {
                    0xf => {
                        if effect.arg_joined() != 0 {
                            current_speed = effect.arg_joined();
                        }
                    }
                    _ => ()
                }
            }
        }
        current_tick += 1;
        if current_tick >= current_speed {
            current_tick = 0;
            current_line += 1;
            processed_line = false;
        }
        if current_line >= 64 {
            current_line = 0;
            current_pattern += 1;
        }
        if current_pattern >= number_of_patterns as usize { break 'main; }
    }
}
