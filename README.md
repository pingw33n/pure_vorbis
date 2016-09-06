# Pure Vorbis

[Vorbis](http://www.vorbis.com/) decoder implementation in pure Rust.

## Documentation

https://docs.rs/pure_vorbis

## Installation

Add it to your `Cargo.toml`:

```toml
[dependencies]
pure_vorbis = "0.0"
```

And to the crate root:

```rust
extern crate pure_vorbis;
```

## Usage

```rust
use std::io::Cursor;
use vorbis::{BitReader, Decoder};

let ident_packet = &[]; // Replace with real data.
let comment_packet = &[]; // Replace with real data.
let setup_packet = &[]; // Replace with real data.

let mut builder = Decoder::builder();
builder.read_ident_packet(&mut BitReader::new(Cursor::new(ident_packet)))
        .expect("Couldn't read ident packet");
builder.read_comment_packet(&mut BitReader::new(Cursor::new(comment_packet)))
        .expect("Couldn't read comment packet");
builder.read_setup_packet(&mut BitReader::new(Cursor::new(setup_packet)))
        .expect("Couldn't read setup packet");
let mut decoder = builder.build();

let mut sample_buf = Vec::with_capacity(decoder.header().frame_lens().long() * decoder.header().channel_count());

loop {
    let audio_packet = &[]; // Replace with real data.
    decoder.decode(&mut BitReader::new(Cursor::new(audio_packet)))
            .expect("Couldn't decode audio packet");
    if decoder.samples().is_empty() {
        continue;
    }
    sample_buf.truncate(0);
    sample_buf.extend(decoder.samples().interleave()
            .map(|s| (s * 32767.0 + 0.5).floor() as i16));

    // Do something with the sample_buf.
}
```

See also the [play example](https://github.com/pingw33n/pure_vorbis/tree/master/examples/play.rs).

## Known issues / limitations

* Floor 0 is not supported.
* Residue 0 is not implemented. Couldn't find or produce a Vorbis file that uses it.
* This implementation is about 2x slower than the reference.

## License

Copyright (c) 2016 Dmytro Lysai

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

- Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.

- Redistributions in binary form must reproduce the above copyright
notice, this list of conditions and the following disclaimer in the
documentation and/or other materials provided with the distribution.

- Neither the name of the original author nor the names of the
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION
OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.