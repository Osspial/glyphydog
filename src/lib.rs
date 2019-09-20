// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate derive_error;

mod hb_funcs;

use std::os::raw::{c_void, c_uint, c_int, c_char};
use freetype::freetype as ft;
use ft::{FT_Face, FT_Library, FT_Error, FT_Size_RequestRec_, FT_ULong, FT_Long};

use harfbuzz_sys::*;

use std::{mem, slice, ptr};
use std::path::Path;
use std::ops::Deref;
use std::ffi::CString;

use euclid::default::{Point2D, Vector2D};

use font_kit::loader::Loader;


pub struct HBFont {
    hb_font: *mut hb_font_t,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapedGlyph {
    pub glyph_index: u32,
    pub advance: Vector2D<i32>,
    pub pos: Point2D<i32>,
    pub str_index: usize,
}

pub struct GlyphSlot<'a> {
    glyph_slot: &'a mut ft::FT_GlyphSlotRec_
}


pub struct ShapedGlyphIter<'a> {
    glyph_iter: std::iter::Zip<std::iter::Cloned<std::slice::Iter<'a, harfbuzz_sys::hb_glyph_position_t>>, std::iter::Cloned<std::slice::Iter<'a, harfbuzz_sys::hb_glyph_info_t>>>,
    cursor: Point2D<i32>,
}

pub trait LoadFontTable: Loader {
    fn load_table(&self, table_tag: u32) -> Option<Box<u8>>;
}

impl HBFont {
    pub fn new<L: LoadFontTable>(loader: &L) -> HBFont {
        unsafe {
            type Table = Option<Box<u8>>;
            unsafe extern "C" fn reference_table<L: LoadFontTable>(_: *mut hb_face_t, tag: hb_tag_t, user_data: *mut c_void) -> *mut hb_blob_t {
                let loader = &*(user_data as *const L);
                let table = loader.load_table(tag);
                let table_ptr = table.map(|table| table.as_mut_ptr()).unwrap_or(ptr::null_mut());
                let table_len = table.map(|table| table.len()).unwrap_or(0);

                assert!(table_len <= c_uint::max_value() as usize);

                let table_box: Box<Table> = Box::new(table);
                let table_ptr: *mut Table = Box::into_raw(table_box);

                hb_blob_create(
                    table_ptr, table_len as c_uint, HB_MEMORY_MODE_WRITABLE,
                    table_ptr as *mut c_void, Some(free_ref_table)
                )
            }
            unsafe extern "C" fn free_ref_table(table: *mut c_void) {
                let table_ptr = table as *mut Table;
                let _: Box<Table> = Box::from_raw(table_ptr);
            }

            // Create the harfbuzz font
            let hb_face = hb_face_create_for_tables(Some(reference_table::<L>), loader as *mut c_void, None);
            hb_face_set_upem(hb_face, (*ft_face).units_per_EM as c_uint);
            let hb_font = hb_font_create(hb_face);
            hb_funcs::set_for_font(hb_font, ft_face);

            // Harfbuzz font creation cleanup
            hb_face_destroy(hb_face);


            Ok(HBFont {
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
        }
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
    pub fn shape_text<'a, B: ?Sized>(
        &'a mut self,
        text: &str,
        face: &mut HBFont<B>,
        face_size: FaceSize,
        dpi: DPI,
    ) -> Result<ShapedGlyphIter<'a>, Error>
    {
        face.resize(face_size, dpi)?;

        let hb_buf = self.hb_buf;

        unsafe{
            // Add the word to the harfbuzz buffer, and shape it.
            hb_buffer_clear_contents(hb_buf);
            hb_buffer_add_utf8(hb_buf, text.as_ptr() as *const c_char, text.len() as i32, 0, text.len() as i32);
            hb_buffer_guess_segment_properties(hb_buf);
            hb_shape(face.hb_font, hb_buf, ptr::null(), 0);
        }


        // Retrieve the pointers to the glyph info from harfbuzz.
        let (mut glyph_info_count, mut glyph_pos_count) = (0, 0);
        let glyph_pos_ptr = unsafe{ hb_buffer_get_glyph_positions(hb_buf, &mut glyph_pos_count) };
        let glyph_info_ptr = unsafe{ hb_buffer_get_glyph_infos(hb_buf, &mut glyph_info_count) };
        assert_eq!(glyph_info_count, glyph_pos_count);
        let glyph_pos = unsafe{ slice::from_raw_parts(glyph_pos_ptr, glyph_pos_count as usize) };
        let glyph_info = unsafe{ slice::from_raw_parts(glyph_info_ptr, glyph_info_count as usize) };

        let shaped_glyph_iter = ShapedGlyphIter {
            glyph_iter: glyph_pos.iter().cloned().zip(glyph_info.iter().cloned()),
            cursor: Point2D::new(0, 0),
        };

        Ok(shaped_glyph_iter)
    }
}

impl Iterator for ShapedGlyphIter<'_> {
    type Item = ShapedGlyph;
    fn next(&mut self) -> Option<ShapedGlyph> {
        let (pos, info) = self.glyph_iter.next()?;

        let glyph_shaped = ShapedGlyph {
            pos: self.cursor + Vector2D::new(pos.x_offset, pos.y_offset),
            advance: Vector2D::new(pos.x_advance, pos.y_advance) / 64,
            glyph_index: info.codepoint,
            str_index: info.cluster as usize,
        };
        self.cursor += Vector2D::new(pos.x_advance, pos.y_advance) / 64;
        Some(glyph_shaped)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.glyph_iter.len(), Some(self.glyph_iter.len()))
    }
}
impl ExactSizeIterator for ShapedGlyphIter<'_> {}

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

impl<B: ?Sized> Drop for HBFont<B> {
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
