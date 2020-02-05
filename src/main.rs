use std::io::{Cursor, Read};
use byteorder::ReadBytesExt;
use arr_macro::arr;
use crate::sample::Sample;
use crate::pattern::Pattern;

mod sample;
mod pattern;

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

    println!("{}", file_tag);
}
