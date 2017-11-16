extern crate gliphydog;

use gliphydog::{FTLib, Face, Shaper, FontSize, DPI};
use std::fs::File;
use std::io::Read;

fn main() {
    let mut font_buf = vec![];
    File::open("./DejaVuSans.ttf").unwrap().read_to_end(&mut font_buf).unwrap();

    let lib = FTLib::new();
    let face = Face::new(font_buf, 0, &lib).unwrap();
    let mut shaper = Shaper::new();

    let font_size = FontSize {
        width: 16*64,
        height: 16*64
    };
    let dpi = DPI {
        hori: 72,
        vert: 72
    };
    for word in shaper.shape_text("Hello World!", &face, font_size, dpi).unwrap() {
        println!("{:#?}", word);
    }
}
