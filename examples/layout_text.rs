extern crate glyphydog;
extern crate cgmath_geometry;
use cgmath_geometry::cgmath;
extern crate png;

use glyphydog::{FTLib, Face, Shaper, FaceSize, DPI, ShapedBuffer, RenderMode, LoadFlags, GlyphMetricsPx};
use std::fs::File;
use std::io::Read;

use std::io::BufWriter;
use png::HasParameters;

use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{GeoBox, OffsetBox, DimsBox};

fn main() {
    let lib = FTLib::new();
    let mut face = Face::new(&include_bytes!("../DejaVuSans.ttf")[..], 0, &lib).unwrap();
    let mut shaper = Shaper::new();

    let mut output_image = vec![0; 256 * 256];

    let font_size = FaceSize {
        width: 16*64,
        height: 16*64
    };
    let dpi = DPI {
        hori: 72,
        vert: 72
    };
    let start_time = ::std::time::Instant::now();
    let mut buffer = ShapedBuffer::new();
    shaper.shape_text("Γειά σου Κόσμε!", &mut face, font_size, dpi, &mut buffer).unwrap();
    shaper.shape_text("Hello World!", &mut face, font_size, dpi, &mut buffer).unwrap();
    let mut cursor_x = 0;
    for i in 0..buffer.segments_len() {
        let segment = buffer.get_segment(i).unwrap();
        for glyph in segment.shaped_glyphs {
            let render_mode = RenderMode::Normal;
            let mut slot = face.load_glyph(glyph.glyph_index, font_size, dpi, LoadFlags::empty(), render_mode).unwrap();
            let bitmap = slot.render_glyph(render_mode).unwrap();
            let metrics = GlyphMetricsPx::from(slot.metrics());

            blit(
                bitmap.buffer, bitmap.dims, bitmap.dims.into(),
                &mut output_image, DimsBox::new2(256, 256),
                    (Vector2::new(cursor_x + metrics.hori_bearing.x, 32 - metrics.hori_bearing.y) + glyph.pos.to_vec()).cast().unwrap()
            );
        }
        cursor_x += segment.advance;
    }
    println!("{:?}", ::std::time::Instant::now() - start_time);

    let file = File::create("./layout_text.png").unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, 256, 256);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&output_image).unwrap();
}

fn blit<P: Copy>(
    src: &[P], src_dims: DimsBox<Point2<u32>>, src_copy_from: OffsetBox<Point2<u32>>,
    dst: &mut [P], dst_dims: DimsBox<Point2<u32>>, dst_offset: Vector2<u32>
) {
    for row_num in 0..src_copy_from.height() as usize {
        let dst_row_num = row_num + dst_offset.y as usize;
        let dst_slice_offset = dst_row_num * dst_dims.width() as usize;
        let dst_row = &mut dst[dst_slice_offset..dst_slice_offset + dst_dims.width() as usize];

        let src_row_num = row_num + src_copy_from.min().y as usize;
        let src_slice_offset = src_row_num * src_dims.width() as usize;
        let src_row = &src[src_slice_offset..src_slice_offset + src_dims.width() as usize];

        let src_copy_slice = &src_row[src_copy_from.min().x as usize..src_copy_from.max().x as usize];
        let dst_copy_to_slice = &mut dst_row[dst_offset.x as usize..(dst_offset.x + src_copy_from.width()) as usize];
        dst_copy_to_slice.copy_from_slice(src_copy_slice);
    }
}
