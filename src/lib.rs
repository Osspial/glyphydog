extern crate harfbuzz_sys;
extern crate freetype;
extern crate libc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate stable_deref_trait;
// extern crate unicode_segmentation;
// extern crate unicode_bidi;
// extern crate unicode_script;
extern crate xi_unicode;
extern crate cgmath;
extern crate cgmath_geometry;
#[macro_use]
extern crate derive_error;

mod hb_funcs;
mod ft_alloc;

use libc::{c_int, c_uint, c_char};
use freetype::freetype as ft;
use ft::{FT_Face, FT_Library, FT_Error, FT_Size_RequestRec_, FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL, FT_ULong, FT_Long};

use harfbuzz_sys::*;

use stable_deref_trait::StableDeref;

use std::{mem, slice, ptr};
use std::path::Path;
use std::ops::{Deref, Range};
use std::ffi::CString;

use cgmath::{Point2, Vector2};
use cgmath_geometry::{DimsBox, GeoBox};

use xi_unicode::LineBreakIterator;


#[derive(Debug)]
pub struct FTLib {
    lib: FT_Library
}

pub struct Face<B> {
    ft_face: FT_Face,
    ft_size_request: FT_Size_RequestRec_,
    hb_font: *mut hb_font_t,
    _font_buffer: B,
    _lib: FTLib
}

pub struct Shaper {
    hb_buf: *mut hb_buffer_t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceSize {
    pub width: u32,
    pub height: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DPI {
    pub hori: u32,
    pub vert: u32
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ShapedBuffer {
    glyphs: Vec<ShapedGlyph>,
    segments: Vec<RawShapedSegment>,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapedSegment<'a> {
    pub text: &'a str,
    pub text_range: Range<usize>,
    pub shaped_glyphs: &'a [ShapedGlyph],
    pub advance: i32,
    pub hard_break: bool
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawShapedSegment {
    text_range: Range<usize>,
    glyph_range: Range<usize>,
    advance: i32,
    hard_break: bool
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapedGlyph {
    pub glyph_index: u32,
    pub advance: Vector2<i32>,
    pub pos: Point2<i32>,
    pub str_index: usize,
    pub word_str_index: usize,
    // pub metrics: GlyphMetricsPx
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphMetrics266 {
    pub dims: DimsBox<Point2<i32>>,
    pub hori_bearing: Vector2<i32>,
    pub hori_advance: i32,
    pub vert_bearing: Vector2<i32>,
    pub vert_advance: i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphMetricsPx {
    pub dims: DimsBox<Point2<i32>>,
    pub hori_bearing: Vector2<i32>,
    pub hori_advance: i32,
    pub vert_bearing: Vector2<i32>,
    pub vert_advance: i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontMetricsFU {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub height: i16,
    pub max_advance_width: i16,
    pub max_advance_height: i16,
    pub underline_position: i16,
    pub underline_thickness: i16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontMetrics266 {
    pub x_ppem: u16,
    pub y_ppem: u16,
    pub x_scale: i32,
    pub y_scale: i32,
    pub ascender: i32,
    pub descender: i32,
    pub height: i32,
    pub max_advance: i32
}

pub struct GlyphSlot<'a> {
    glyph_slot: &'a mut ft::FT_GlyphSlotRec_
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitmap<'a> {
    pub dims: DimsBox<Point2<u32>>,
    pub pitch: i32,
    pub buffer: &'a [u8],
    pub pixel_mode: PixelMode
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelMode {
    Mono,
    Gray,
    Gray2,
    Gray4,
    Lcd,
    LcdV,
    Bgra
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderMode {
    Normal = ft::FT_Render_Mode__FT_RENDER_MODE_NORMAL as isize,
    Light = ft::FT_Render_Mode__FT_RENDER_MODE_LIGHT as isize,
    Mono = ft::FT_Render_Mode__FT_RENDER_MODE_MONO as isize,
    Lcd = ft::FT_Render_Mode__FT_RENDER_MODE_LCD as isize,
    LcdV = ft::FT_Render_Mode__FT_RENDER_MODE_LCD_V as isize,
}

bitflags!{
    pub struct LoadFlags: c_uint {
        const NO_SCALE = ft::FT_LOAD_NO_SCALE;
        const NO_HINTING = ft::FT_LOAD_NO_HINTING;
        const RENDER = ft::FT_LOAD_RENDER;
        const NO_BITMAP = ft::FT_LOAD_NO_BITMAP;
        const VERTICAL_LAYOUT = ft::FT_LOAD_VERTICAL_LAYOUT;
        const FORCE_AUTOHINT = ft::FT_LOAD_FORCE_AUTOHINT;
        const CROP_BITMAP = ft::FT_LOAD_CROP_BITMAP;
        const PEDANTIC = ft::FT_LOAD_PEDANTIC;
        const IGNORE_GLOBAL_ADVANCE_WIDTH = ft::FT_LOAD_IGNORE_GLOBAL_ADVANCE_WIDTH;
        const NO_RECURSE = ft::FT_LOAD_NO_RECURSE;
        const IGNORE_TRANSFORM = ft::FT_LOAD_IGNORE_TRANSFORM;
        const MONOCHROME = ft::FT_LOAD_MONOCHROME;
        const LINEAR_DESIGN = ft::FT_LOAD_LINEAR_DESIGN;
        const NO_AUTOHINT = ft::FT_LOAD_NO_AUTOHINT;
        const COLOR = ft::FT_LOAD_COLOR;
        const COMPUTE_METRICS = ft::FT_LOAD_COMPUTE_METRICS;
        const BITMAP_METRICS_ONLY = ft::FT_LOAD_BITMAP_METRICS_ONLY;
    }
}


impl FTLib {
    pub fn new() -> FTLib {
        let mut lib = ptr::null_mut();
        unsafe {
            assert_eq!(FT_Error(0), ft::FT_New_Library(ft_alloc::alloc_mem_rec(), &mut lib));
            ft::FT_Add_Default_Modules(lib);
        }

        FTLib{ lib }
    }
}

impl Face<()> {
    pub fn new_path<P: AsRef<Path>>(path: P, face_index: i32, lib: &FTLib) -> Result<Face<()>, Error> {
        unsafe {
            let path_canon = path.as_ref().canonicalize().map_err(|_| Error::CannotOpenResource)?;
            let path_c = CString::new(path_canon.to_str().ok_or(Error::CannotOpenResource)?.to_owned()).map_err(|_| Error::CannotOpenResource)?;

            let mut ft_face = ptr::null_mut();

            // Allocate the face in freetype, and ensure that it was created successfully
            let err_raw = ft::FT_New_Face(
                lib.lib,
                path_c.as_ptr(),
                face_index as FT_Long,
                &mut ft_face
            );

            match Error::from_raw(err_raw).unwrap() {
                Error::Ok => {
                    use libc::c_void;
                    unsafe extern "C" fn reference_table(_: *mut hb_face_t, tag: hb_tag_t, user_data: *mut c_void) -> *mut hb_blob_t {
                        let ft_face = user_data as FT_Face;
                        let mut len = 0;

                        if FT_Error(0) != ft::FT_Load_Sfnt_Table(ft_face, tag as FT_ULong, 0, ptr::null_mut(), &mut len) {
                            return ptr::null_mut();
                        }

                        let mut buf = vec![0; len as usize];
                        if FT_Error(0) != ft::FT_Load_Sfnt_Table(ft_face, tag as FT_ULong, 0, buf.as_mut_ptr() as *mut ft::FT_Byte, &mut len) {
                            return ptr::null_mut();
                        }

                        hb_blob_create(
                            buf.as_mut_ptr(), len as c_uint, HB_MEMORY_MODE_WRITABLE,
                            Box::into_raw(Box::new(buf)) as *mut c_void, Some(free_ref_table)
                        )
                    }
                    unsafe extern "C" fn free_ref_table(table: *mut c_void) {
                        Box::from_raw(table as *mut Vec<c_char>);
                    }

                    // Create the harfbuzz font
                    let hb_face = hb_face_create_for_tables(Some(reference_table), ft_face as *mut c_void, None);
                    hb_face_set_upem(hb_face, (*ft_face).units_per_EM as c_uint);
                    let hb_font = hb_font_create(hb_face);
                    hb_funcs::set_for_font(hb_font, ft_face);

                    // Harfbuzz font creation cleanup
                    hb_face_destroy(hb_face);


                    Ok(Face {
                        ft_face,
                        hb_font,
                        ft_size_request: FT_Size_RequestRec_ {
                            type_: FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL,
                            width: -1,
                            height: -1,
                            horiResolution: 0,
                            vertResolution: 0
                        },

                        _font_buffer: (),
                        _lib: lib.clone()
                    })
                },
                err => Err(err)
            }
        }
    }
}

impl<B> Face<B>
    where B: StableDeref + Deref<Target=[u8]>
{
    pub fn new(font_buffer: B, face_index: i32, lib: &FTLib) -> Result<Face<B>, Error> {
        let mut ft_face = ptr::null_mut();
        unsafe {
            // Allocate the face in freetype, and ensure that it was created successfully
            let err_raw = ft::FT_New_Memory_Face(
                lib.lib,
                font_buffer.as_ptr(),
                font_buffer.len() as FT_Long,
                face_index as FT_Long,
                &mut ft_face
            );

            match Error::from_raw(err_raw).unwrap() {
                Error::Ok => {
                    // Create the harfbuzz font
                    let hb_blob = hb_blob_create(
                        font_buffer.as_ptr() as *const c_char,
                        font_buffer.len() as c_uint,
                        HB_MEMORY_MODE_READONLY,
                        ptr::null_mut(),
                        None
                    );
                    let hb_face = hb_face_create(hb_blob, face_index as c_uint);
                    hb_face_set_upem(hb_face, (*ft_face).units_per_EM as c_uint);
                    let hb_font = hb_font_create(hb_face);
                    hb_funcs::set_for_font(hb_font, ft_face);

                    // Harfbuzz font creation cleanup
                    hb_face_destroy(hb_face);
                    hb_blob_destroy(hb_blob);


                    Ok(Face {
                        ft_face,
                        hb_font,
                        ft_size_request: FT_Size_RequestRec_ {
                            type_: FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL,
                            width: -1,
                            height: -1,
                            horiResolution: 0,
                            vertResolution: 0
                        },

                        _font_buffer: font_buffer,
                        _lib: lib.clone()
                    })
                },
                err => Err(err)
            }
        }
    }
}

impl<B> Face<B> {
    pub fn load_glyph<'a>(
        &'a mut self,
        glyph_index: u32,
        face_size: FaceSize,
        dpi: DPI,
        load_flags: LoadFlags,
        hint_algo: RenderMode
    ) -> Result<GlyphSlot<'a>, Error>
    {
        self.resize(face_size, dpi)?;
        let mut load_flags = load_flags.bits as c_int;

        unsafe {
            load_flags |= (mem::transmute::<_, c_int>(hint_algo) & 16) << 16;

            let error = ft::FT_Load_Glyph(self.ft_face, glyph_index, load_flags);
            match error {
                FT_Error(0) => Ok(GlyphSlot {
                    glyph_slot: &mut *(*self.ft_face).glyph
                }),
                FT_Error(_) => Err(Error::from_raw(error).unwrap())
            }
        }
    }

    #[inline]
    pub fn char_index(&self, c: char) -> u32 {
        unsafe{ ft::FT_Get_Char_Index(self.ft_face, c as FT_ULong) }
    }

    /// Retrieve the glyph advance. If scaling is performed (based on the value of `load_flags`),
    /// the advance value is in 16.16 format. Otherwise, it is in font units.
    pub fn glyph_advance(&mut self, glyph_index: u32, face_size: FaceSize, dpi: DPI, load_flags: LoadFlags) -> Result<i32, Error> {
        if !load_flags.contains(LoadFlags::NO_SCALE) {
            self.resize(face_size, dpi)?;
        }

        unsafe {
            let mut advance = 0;
            let error = ft::FT_Get_Advance(self.ft_face, glyph_index, mem::transmute(load_flags), &mut advance);
            match error {
                FT_Error(0) => Ok(advance as i32),
                _ => Err(Error::from_raw(error).unwrap())
            }
        }
    }

    #[inline]
    pub fn metrics_font_units(&self) -> FontMetricsFU {
        let ft_face_ref = unsafe{ &*self.ft_face };
        FontMetricsFU {
            units_per_em: ft_face_ref.units_per_EM,
            ascender: ft_face_ref.ascender,
            descender: ft_face_ref.descender,
            height: ft_face_ref.height,
            max_advance_width: ft_face_ref.max_advance_width,
            max_advance_height: ft_face_ref.max_advance_height,
            underline_position: ft_face_ref.underline_position,
            underline_thickness: ft_face_ref.underline_thickness,
        }
    }

    #[inline]
    pub fn metrics_sized(&mut self, face_size: FaceSize, dpi: DPI) -> Result<FontMetrics266, Error> {
        self.resize(face_size, dpi)?;

        let size_metrics = unsafe{ &(*(*self.ft_face).size).metrics };
        Ok(FontMetrics266 {
            x_ppem: size_metrics.x_ppem,
            y_ppem: size_metrics.y_ppem,
            x_scale: size_metrics.x_scale as i32,
            y_scale: size_metrics.y_scale as i32,
            ascender: size_metrics.ascender as i32,
            descender: size_metrics.descender as i32,
            height: size_metrics.height as i32,
            max_advance: size_metrics.max_advance as i32,
        })
    }

    fn resize(&mut self, face_size: FaceSize, dpi: DPI) -> Result<(), Error> {
        // Determine if we need to change the freetype font size, and change it if necessary
        let old_size_request = (
            FaceSize {
                width: self.ft_size_request.width as u32,
                height: self.ft_size_request.height as u32
            },
            DPI {
                hori: self.ft_size_request.horiResolution,
                vert: self.ft_size_request.vertResolution
            }
        );
        if (face_size, dpi) != old_size_request {
            // Change freetype font size
            let mut size_request = FT_Size_RequestRec_ {
                type_: FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL,
                width: face_size.width as FT_Long,
                height: face_size.height as FT_Long,
                horiResolution: dpi.hori,
                vertResolution: dpi.vert
            };
            let error = unsafe{ ft::FT_Request_Size(self.ft_face, &mut size_request) };
            if FT_Error(0) != error {
                return Err(Error::from_raw(error).unwrap());
            }
        }
        Ok(())
    }
}

impl Shaper {
    pub fn new() -> Shaper {
        unsafe {
            Shaper {
                hb_buf: hb_buffer_create()
            }
        }
    }

    #[inline]
    pub fn shape_text<B>(&mut self,
        text: &str,
        face: &mut Face<B>,
        face_size: FaceSize,
        dpi: DPI,
        buffer: &mut ShapedBuffer
    ) -> Result<(), Error> {
        face.resize(face_size, dpi)?;

        let text_offset = buffer.text.len();
        buffer.text.push_str(text);

        let mut last_break = 0;
        for (break_index, hard_break) in LineBreakIterator::new(text) {
            let segment_str = &text[last_break..break_index];
            let hb_buf = self.hb_buf;

            unsafe{
                // Add the word to the harfbuzz buffer, and shape it.
                hb_buffer_clear_contents(hb_buf);
                hb_buffer_add_utf8(hb_buf, segment_str.as_ptr() as *const c_char, segment_str.len() as i32, 0, segment_str.len() as i32);
                hb_buffer_guess_segment_properties(hb_buf);
                hb_shape(face.hb_font, hb_buf, ptr::null(), 0);
            }


            // Retrieve the pointers to the glyph info from harfbuzz.
            let (mut glyph_info_count, mut glyph_pos_count) = (0, 0);
            let glyph_pos_ptr = unsafe{ hb_buffer_get_glyph_positions(hb_buf, &mut glyph_pos_count) };
            let glyph_info_ptr = unsafe{ hb_buffer_get_glyph_infos(hb_buf, &mut glyph_info_count) };
            assert_eq!(glyph_info_count, glyph_pos_count);

            // Transform harfbuzz's glyph info into the rusty format, and add them to the buffer.
            let mut cursor = Point2::new(0, 0);
            let mut glyph_range = 0..0;
            {
                let glyph_info_iter = (0..glyph_pos_count as isize).map(|i| {
                    let pos = unsafe{ *glyph_pos_ptr.offset(i) };
                    let info = unsafe{ *glyph_info_ptr.offset(i) };

                    // let glyph_metrics = unsafe{ match ft::FT_Load_Glyph(face.ft_face, info.codepoint, 0) {
                    //     FT_Error(0) => {
                    //         let ft_metrics = (*(*face.ft_face).glyph).metrics;
                    //         GlyphMetricsPx {
                    //             dims: DimsBox::new2((ft_metrics.width / 64) as i32, (ft_metrics.height / 64) as i32),
                    //             hori_bearing: Vector2::new((ft_metrics.horiBearingX / 64) as i32, (ft_metrics.horiBearingY / 64) as i32),
                    //             hori_advance: (ft_metrics.vertAdvance / 64) as i32
                    //         }
                    //     },
                    //     _ => mem::zeroed()
                    // } };

                    let glyph_shaping = ShapedGlyph {
                        pos: cursor + Vector2::new(pos.x_offset, pos.y_offset),
                        advance: Vector2::new(pos.x_advance, pos.y_advance) / 64,
                        glyph_index: info.codepoint,
                        str_index: last_break + info.cluster as usize,
                        word_str_index: info.cluster as usize,
                        // metrics: glyph_metrics
                    };
                    cursor += Vector2::new(pos.x_advance, pos.y_advance) / 64;
                    glyph_shaping
                });
                glyph_range.start = buffer.glyphs.len();
                buffer.glyphs.extend(glyph_info_iter);
                glyph_range.end = buffer.glyphs.len();
            }

            buffer.segments.push(RawShapedSegment {
                text_range: last_break + text_offset..break_index + text_offset,
                glyph_range,
                advance: cursor.x,
                hard_break
            });
            last_break = break_index;
        }

        Ok(())
    }
}

impl ShapedBuffer {
    #[inline]
    pub fn new() -> ShapedBuffer {
        ShapedBuffer::default()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.segments.clear();
        self.text.clear();
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.glyphs.shrink_to_fit();
        self.segments.shrink_to_fit();
        self.text.shrink_to_fit();
    }

    #[inline]
    pub fn segments_len(&self) -> usize {
        self.segments.len()
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn get_segment<'a>(&'a self, index: usize) -> Option<ShapedSegment<'a>> {
        self.segments.get(index).cloned().map(|s| ShapedSegment {
            text: &self.text.get(s.text_range.clone()).expect("bad text"),
            text_range: s.text_range,
            shaped_glyphs: &self.glyphs.get(s.glyph_range).expect("bad range"),
            advance: s.advance,
            hard_break: s.hard_break
        })
    }
}

impl<'a> GlyphSlot<'a> {
    pub fn metrics(&self) -> GlyphMetrics266 {
        let ft_metrics = self.glyph_slot.metrics;

        GlyphMetrics266 {
            dims: DimsBox::new2(ft_metrics.width as i32, ft_metrics.height as i32),
            hori_bearing: Vector2::new(ft_metrics.horiBearingX as i32, ft_metrics.horiBearingY as i32),
            hori_advance: ft_metrics.horiAdvance as i32,
            vert_bearing: Vector2::new(ft_metrics.vertBearingX as i32, ft_metrics.vertBearingY as i32),
            vert_advance: ft_metrics.vertAdvance as i32,
        }
    }

    pub fn render_glyph(&mut self, render_mode: RenderMode) -> Result<Bitmap<'a>, Error> {
        unsafe {
            let ft_render_mode = mem::transmute(render_mode);
            match ft::FT_Render_Glyph(self.glyph_slot, ft_render_mode) {
                FT_Error(0) => Ok(self.bitmap().expect("bad bitmap")),
                error => Err(Error::from_raw(error).unwrap())
            }
        }
    }

    pub fn bitmap(&self) -> Option<Bitmap<'a>> {
        let ft_bitmap = self.glyph_slot.bitmap;
        match ft_bitmap.pixel_mode {
            0 => None,
            _ => Some(Bitmap {
                dims: DimsBox::new2(ft_bitmap.width as u32, ft_bitmap.rows as u32),
                pitch: ft_bitmap.pitch,
                buffer: match ft_bitmap.buffer as usize {
                    // If we just returned a from_raw_parts when the buffer was null, the null pointer
                    // optimization would kick in and turn the `Some` into a `None`.
                    0x0 => &[],
                    _ => unsafe{ slice::from_raw_parts(ft_bitmap.buffer, (ft_bitmap.pitch.abs() as u32 * ft_bitmap.rows) as usize) }
                },
                pixel_mode: match ft_bitmap.pixel_mode as c_int {
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_MONO  => PixelMode::Mono,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_GRAY  => PixelMode::Gray,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_GRAY2 => PixelMode::Gray2,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_GRAY4 => PixelMode::Gray4,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_LCD   => PixelMode::Lcd,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_LCD_V => PixelMode::LcdV,
                    ft::FT_Pixel_Mode__FT_PIXEL_MODE_BGRA  => PixelMode::Bgra,
                    _ => return None
                }
            })
        }
    }
}

impl FaceSize {
    #[inline]
    pub fn new(width: u32, height: u32) -> FaceSize {
        FaceSize{ width, height }
    }
}

impl DPI {
    #[inline]
    pub fn new(hori: u32, vert: u32) -> DPI {
        DPI{ hori, vert }
    }
}

impl From<GlyphMetrics266> for GlyphMetricsPx {
    fn from(metrics: GlyphMetrics266) -> GlyphMetricsPx {
        GlyphMetricsPx {
            dims: DimsBox::new2(metrics.dims.width() / 64, metrics.dims.height() / 64),
            hori_bearing: metrics.hori_bearing / 64,
            hori_advance: metrics.hori_advance / 64,
            vert_bearing: metrics.vert_bearing / 64,
            vert_advance: metrics.vert_advance / 64,
        }
    }
}

impl Clone for FTLib {
    fn clone(&self) -> FTLib {
        unsafe{ ft::FT_Reference_Library(self.lib) };
        FTLib {
            lib: self.lib
        }
    }
}

impl<B> Clone for Face<B>
    where B: StableDeref + Deref<Target=[u8]> + Clone
{
    fn clone(&self) -> Face<B> {
        let buf = self._font_buffer.clone();
        Face::new(buf, unsafe{ (*self.ft_face).face_index as i32 }, &self._lib).unwrap()
    }
}

impl Drop for FTLib {
    fn drop(&mut self) {
        unsafe{ ft::FT_Done_Library(self.lib) };
    }
}

impl<B> Drop for Face<B> {
    fn drop(&mut self) {
        unsafe {
            hb_font_destroy(self.hb_font);
            ft::FT_Done_Face(self.ft_face);
        }
    }
}

impl Drop for Shaper {
    fn drop(&mut self) {
        unsafe {
            hb_buffer_destroy(self.hb_buf);
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum Error {
    Ok = 0,
    CannotOpenResource = 1,
    UnknownFileFormat = 2,
    InvalidFileFormat = 3,
    InvalidVersion = 4,
    LowerModuleVersion = 5,
    InvalidArgument = 6,
    UnimplementedFeature = 7,
    InvalidTable = 8,
    InvalidOffset = 9,
    ArrayTooLarge = 10,
    MissingModule = 11,
    MissingProperty = 12,
    InvalidGlyphIndex = 16,
    InvalidCharacterCode = 17,
    InvalidGlyphFormat = 18,
    CannotRenderGlyph = 19,
    InvalidOutline = 20,
    InvalidComposite = 21,
    TooManyHints = 22,
    InvalidPixelSize = 23,
    InvalidHandle = 32,
    InvalidLibraryHandle = 33,
    InvalidDriverHandle = 34,
    InvalidFaceHandle = 35,
    InvalidSizeHandle = 36,
    InvalidSlotHandle = 37,
    InvalidCharMapHandle = 38,
    InvalidCacheHandle = 39,
    InvalidStreamHandle = 40,

    TooManyDrivers = 48,
    TooManyExtensions = 49,

    OutOfMemory = 64,
    UnlistedObject = 65,

    CannotOpenStream = 81,
    InvalidStreamSeek = 82,
    InvalidStreamSkip = 83,
    InvalidStreamRead = 84,
    InvalidStreamOperation = 85,
    InvalidFrameOperation = 86,
    NestedFrameAccess = 87,
    InvalidFrameRead = 88,

    RasterUninitialized = 96,
    RasterCorrupted = 97,
    RasterOverflow = 98,
    RasterNegativeHeight = 99,

    TooManyCaches = 112,

    InvalidOpcode = 128,
    TooFewArguments = 129,
    StackOverflow = 130,
    CodeOverflow = 131,
    BadArgument = 132,
    DivideByZero = 133,
    InvalidReference = 134,
    DebugOpCode = 135,
    ENDFInExecStream = 136,
    NestedDEFS = 137,
    InvalidCodeRange = 138,
    ExecutionTooLong = 139,
    TooManyFunctionDefs = 140,
    TooManyInstructionDefs = 141,
    TableMissing = 142,
    HorizHeaderMissing = 143,
    LocationsMissing = 144,
    NameTableMissing = 145,
    CMapTableMissing = 146,
    HmtxTableMissing = 147,
    PostTableMissing = 148,
    InvalidHorizMetrics = 149,
    InvalidCharMapFormat = 150,
    InvalidPPem = 151,
    InvalidVertMetrics = 152,
    CouldNotFindContext = 153,
    InvalidPostTableFormat = 154,
    InvalidPostTable = 155,
    DEFInGlyfBytecode = 156,
    MissingBitmap = 157,
    SyntaxError = 160,
    StackUnderflow = 161,
    Ignore = 162,
    NoUnicodeGlyphName = 163,
    GlyphTooBig = 164,

    MissingStartfontField = 176,
    MissingFontField = 177,
    MissingSizeField = 178,
    MissingFontboundingboxField = 179,
    MissingCharsField = 180,
    MissingStartcharField = 181,
    MissingEncodingField = 182,
    MissingBbxField = 183,
    BbxTooBig = 184,
    CorruptedFontHeader = 185,
    CorruptedFontGlyphs = 186,
    Max = 187,
}

impl Error {
    pub fn from_raw(err: FT_Error) -> Option<Error> {
        let err_in_bounds = move |left, right| left <= err.0 && err.0 <= right;

        // Make sure that the error is valid before transmuting it.
        let eib =
            err_in_bounds(0, 49) ||
            err_in_bounds(64, 65) ||
            err_in_bounds(81, 88) ||
            err_in_bounds(96, 99) ||
            err.0 == 112 ||
            err_in_bounds(128, 164) ||
            err_in_bounds(176, 187);

        if eib {
            Some(unsafe{ mem::transmute(err)})
        } else {
            None
        }
    }
}
