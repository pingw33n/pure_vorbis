extern crate criterion;
extern crate ogg_vorbis_ref;
extern crate vorbis;

use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use ogg_vorbis_ref::{OggRefDecoder, VorbisRefDecoder};
use vorbis::{BitReader, Decoder};

use criterion::Criterion;

struct DecodeRef {
    ogg: OggRefDecoder<File>,
    decoder: VorbisRefDecoder,
}

impl DecodeRef {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).unwrap();

        let mut ogg = OggRefDecoder::new(file, 16384);

        let mut decoder = VorbisRefDecoder::new();

        ogg.next_packet().unwrap();
        decoder.decode_header(ogg.raw_packet_mut()).unwrap();

        ogg.next_packet().unwrap();
        decoder.decode_header(ogg.raw_packet_mut()).unwrap();

        ogg.next_packet().unwrap();
        decoder.decode_header(ogg.raw_packet_mut()).unwrap();

        DecodeRef {
            ogg: ogg,
            decoder: decoder,
        }
    }

    pub fn bench(&mut self) {
        while self.ogg.next_packet().unwrap() {
            self.decoder.decode(self.ogg.raw_packet_mut()).unwrap();
        }
    }
}

#[test] #[ignore]
fn bench_decode_ref() {
    Criterion::default().bench_function("decode_ref", |b| b.iter_with_setup(
        || DecodeRef::new(bench_file()),
        |mut d| d.bench()
    )).summarize("decode_ref");
}

struct DecodeSelf {
    ogg: OggRefDecoder<File>,
    decoder: Decoder,
}

impl DecodeSelf {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).unwrap();

        let mut ogg = OggRefDecoder::new(file, 16384);

        let mut decoder_builder = Decoder::builder();

        ogg.next_packet().unwrap();
        decoder_builder.read_ident_packet(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

        ogg.next_packet().unwrap();

        ogg.next_packet().unwrap();
        decoder_builder.read_setup_packet(&mut BitReader::new(Cursor::new(ogg.packet_data()))).unwrap();

        let decoder = decoder_builder.build();

        DecodeSelf {
            ogg: ogg,
            decoder: decoder,
        }
    }

    pub fn bench(&mut self) {
        while self.ogg.next_packet().unwrap() {
            self.decoder.decode(&mut BitReader::new(Cursor::new(self.ogg.packet_data()))).unwrap();
        }
    }
}

#[test] #[ignore]
fn bench_decode_self() {
    Criterion::default().bench_function("decode_self", |b| b.iter_with_setup(
        || DecodeSelf::new(bench_file()),
        |mut d| d.bench()
    )).summarize("decode_self");
}

fn bench_file() -> PathBuf {
    let mut r = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    r.push("data/bench.ogg");
    r
}