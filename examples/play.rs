extern crate ao;
extern crate clap;
extern crate ogg_vorbis_ref;
extern crate vorbis;

use ao::{AO, Endianness, SampleFormat};
use clap::{Arg, App};
use std::fs::File;
use std::io::Cursor;

use ogg_vorbis_ref::OggRefDecoder;
use vorbis::{BitReader, CommentTag, Decoder};

fn main() {
    let matches = App::new("Pure Vorbis Player")
                    .about("Demonstrates usage of the Pure Vorbis decoder library.\
                            Can stream decoded OGG Vorbis data to an audio device or a file")
                    .arg(Arg::with_name("INPUT")
                        .help("Specifies the OGG Vorbis file to play")
                        .required(true))
                    .get_matches();
    let path = matches.value_of("INPUT").unwrap();

    let file = File::open(path).expect("Couldn't open input file");

    println!("Playing {}", path);

    let mut ogg = OggRefDecoder::new(file, 4096);

    let mut decoder_builder = Decoder::builder();

    ogg.next_packet().expect("Couldn't read ident packet");
    decoder_builder.read_ident_packet(&mut BitReader::new(Cursor::new(ogg.packet_data())))
            .expect("Couldn't decode ident packet");
    {
        let header = decoder_builder.header().unwrap();
        println!("Channels: {}", header.channel_count());
        println!("Sample rate: {}", header.sample_rate());
        println!("Bitrate (min / nom / max): {} / {} / {}",
                header.bitrates().min(), header.bitrates().nom(), header.bitrates().max());
        println!("Frame lengths (short / long): {} / {}",
                header.frame_lens().short(), header.frame_lens().long());
    }

    ogg.next_packet().expect("Couldn't read comment packet");
    decoder_builder.read_comment_packet(&mut BitReader::new(Cursor::new(ogg.packet_data())))
            .expect("Couldn't decode comment packet");
    {
        let comments = decoder_builder.comments().unwrap();
        println!("Comments:");
        println!("  Vendor: {}", comments.vendor().unwrap_or(""));
        for (tag, val) in comments {
            println!("  {}: {}", CommentTag::from(tag), if val.len() < 50 { val } else { "<value is too long>" });
        }
    }

    ogg.next_packet().expect("Couldn't read setup packet");
    decoder_builder.read_setup_packet(&mut BitReader::new(Cursor::new(ogg.packet_data())))
            .expect("Couldn't decode setup packet");

    let mut decoder = decoder_builder.build();

    let ao = AO::init();
    let format = SampleFormat::<i16, &str>::new(
            decoder.header().sample_rate() as usize,
            decoder.header().channel_count(),
            Endianness::Native, None);
    let drv = ao.get_driver("").expect("Couldn't get audio driver");
    let dev = drv.open_live(&format).expect("Couldn't open audio device");

    let mut buf = Vec::with_capacity(decoder.header().frame_lens().long() * decoder.header().channel_count());

    while ogg.next_packet().expect("Couldn't read audio packet") {
        decoder.decode(&mut BitReader::new(Cursor::new(ogg.packet_data()))).expect("Couldn't decode audio packet");
        if decoder.samples().is_empty() {
            continue;
        }
        buf.truncate(0);
        buf.extend(decoder.samples().interleave()
            .map(|s| (s * 32767.0 + 0.5).floor() as i16));
        dev.play(&buf);
    }
}
