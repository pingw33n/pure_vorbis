extern crate num_cpus;
extern crate ogg_vorbis_ref;
extern crate scoped_pool;
extern crate vorbis;

use scoped_pool::Pool;
use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use ogg_vorbis_ref::{OggRefDecoder, VorbisRefDecoder};
use vorbis::{Decoder, BitReader};

#[test] #[ignore]
fn ref_test() {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/data/ref");
    let thread_pool = Pool::new(num_cpus::get());

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.to_string_lossy().ends_with(".ogg") {
            println!("> Spawning reference test: {}", path.file_name().unwrap().to_string_lossy());
            thread_pool.spawn(|| do_ref_test(path));
        }
    }

    thread_pool.shutdown();
}

fn do_ref_test<P: AsRef<Path>>(path: P) {
    let file = File::open(path).unwrap();

    let mut ogg = OggRefDecoder::new(file, 4096);

    let mut ref_decoder = VorbisRefDecoder::new();
    let mut decoder_builder = Decoder::builder();

    ogg.next_packet().unwrap();
    ref_decoder.decode_header(ogg.raw_packet_mut()).unwrap();
    decoder_builder.read_ident_packet(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

    ogg.next_packet().unwrap();
    ref_decoder.decode_header(ogg.raw_packet_mut()).unwrap();
    decoder_builder.read_comment_packet(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

    {
        let actual = decoder_builder.comments().unwrap();
        assert_eq!(actual.vendor(), ref_decoder.comment_vendor());
        assert_eq!(actual.len(), ref_decoder.comment_count());
        for i in 0..ref_decoder.comment_count() {
            assert_eq!(actual.raw()[i], ref_decoder.comment(i).unwrap());
        }
    }

    ogg.next_packet().unwrap();
    ref_decoder.decode_header(ogg.raw_packet_mut()).unwrap();
    decoder_builder.read_setup_packet(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

    let mut decoder = decoder_builder.build();

    assert_eq!(decoder.header().channel_count(), ref_decoder.channel_count());

    while ogg.next_packet().unwrap() {
        ref_decoder.decode(ogg.raw_packet_mut()).unwrap();

        let actual = decoder.decode(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

        for ch in 0..ref_decoder.channel_count() {
            let expected = ref_decoder.pcm(ch);

            let actual = actual.channel(ch);
            let actual = if ogg.is_eos() {
                &actual[..expected.len()]
            } else {
                actual
            };
            assert!(expected.len() == actual.len());

            for (&e, &a) in expected.iter().zip(actual.iter()) {
                let eq = (e - a).abs() < 1e-6;
                if !eq {
                    println!("actual {} != expected {}", a, e);
                }
                assert!(eq);
            }
        }
    }
}