extern crate harfbuzz_sys;
extern crate freetype;
extern crate libc;
#[macro_use]
extern crate lazy_static;
extern crate stable_deref_trait;
extern crate typed_arena;
// extern crate unicode_segmentation;
// extern crate unicode_bidi;
// extern crate unicode_script;
extern crate xi_unicode;
extern crate cgmath;

mod hb_funcs;
mod ft_alloc;

use libc::{c_int, c_uint, c_char};
use freetype::freetype as ft;
use ft::{FT_Face, FT_Library, FT_Error, FT_Size_RequestRec_, FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL};

use harfbuzz_sys::*;

use stable_deref_trait::StableDeref;

use std::{mem, ptr};
use std::ops::Deref;

use cgmath::{Point2, Vector2};

use typed_arena::Arena;
use xi_unicode::LineBreakIterator;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphData {
    /// The position to draw the glyph in, relative to the start of the word.
    pub pos: Point2<i32>,
    /// The glyph index.
    pub glyph_index: u32,
    /// The byte offset from the beginning of the text where the character drawing this
    /// glyph begins.
    pub str_index: usize
}

#[derive(Debug)]
pub struct FTLib {
    lib: FT_Library
}

pub struct Face<B>
    where B: StableDeref + Deref<Target=[u8]>
{
    ft_face: FT_Face,
    ft_size_request: FT_Size_RequestRec_,
    hb_font: *mut hb_font_t,
    _font_buffer: B,
    _lib: FTLib
}

pub struct Shaper {
    hb_buf: *mut hb_buffer_t,
    glyph_arena: Arena<GlyphData>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontSize {
    pub width: u32,
    pub height: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DPI {
    pub hori: u32,
    pub vert: u32
}

/// TODO: HANDLE BIDI TEXT
pub struct WordIter<'a> {
    line_breaks: LineBreakIterator<'a>,
    last_break: usize,
    text: &'a str,
    shaper: &'a Shaper,
    // Borrowed from a `Face`
    hb_font: *mut hb_font_t,
    // Borrowed from a `Face`
    ft_face: FT_Face
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Word<'a> {
    pub text: &'a str,
    pub glyphs: &'a [GlyphData],
    /// The pixel advance of the word.
    pub advance: i32,
    /// Whether or not a line break is required after this word.
    pub hard_break: bool,
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
                font_buffer.len() as c_int,
                face_index,
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

impl Shaper {
    pub fn new() -> Shaper {
        unsafe {
            Shaper {
                hb_buf: hb_buffer_create(),
                glyph_arena: Arena::new()
            }
        }
    }

    #[inline]
    pub fn shape_text<'a, B>(&'a mut self,
        text: &'a str,
        face: &'a Face<B>,
        font_size: FontSize,
        dpi: DPI
    ) -> Result<WordIter<'a>, Error>
        where B: StableDeref + Deref<Target=[u8]>
    {
        // Determine if we need to change the freetype font size, and change it if necessary
        let old_size_request = (
            FontSize {
                width: face.ft_size_request.width as u32,
                height: face.ft_size_request.height as u32
            },
            DPI {
                hori: face.ft_size_request.horiResolution,
                vert: face.ft_size_request.vertResolution
            }
        );
        if (font_size, dpi) != old_size_request {
            // Change freetype font size
            let mut size_request = FT_Size_RequestRec_ {
                type_: FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL,
                width: font_size.width as i32,
                height: font_size.height as i32,
                horiResolution: dpi.hori,
                vertResolution: dpi.vert
            };
            let error = unsafe{ ft::FT_Request_Size(face.ft_face, &mut size_request) };
            if FT_Error(0) != error {
                return Err(Error::from_raw(error).unwrap());
            }
        }

        Ok(WordIter {
            line_breaks: LineBreakIterator::new(text),
            last_break: 0,
            text,
            shaper: self,
            hb_font: face.hb_font,
            ft_face: face.ft_face
        })
    }
}

impl<'a> Iterator for WordIter<'a> {
    type Item = Word<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Word<'a>> {
        let WordIter {
            ref mut last_break,
            ref mut line_breaks,
            text,
            shaper,
            hb_font,
            ft_face
        } = *self;

        // Transform the break index into a string slice, advancing the last_break index to the
        // new line break.
        let next_word = line_breaks.next()
            .map(|(break_index, hard_break)| {
                let lb = *last_break;
                *last_break = break_index;

                (&text[lb..break_index], hard_break)
            });

        match next_word {
            Some((word, hard_break)) => unsafe {
                let hb_buf = shaper.hb_buf;

                // Add the word to the harfbuzz buffer, and shape it.
                hb_buffer_clear_contents(hb_buf);
                hb_buffer_add_utf8(hb_buf, word.as_ptr() as *const c_char, word.len() as i32, 0, word.len() as i32);
                hb_buffer_guess_segment_properties(hb_buf);
                hb_shape(hb_font, hb_buf, ptr::null(), 0);


                // Retrieve the pointers to the glyph info from harfbuzz.
                let (mut glyph_info_count, mut glyph_pos_count) = (0, 0);
                let glyph_pos_ptr = hb_buffer_get_glyph_positions(hb_buf, &mut glyph_pos_count);
                let glyph_info_ptr = hb_buffer_get_glyph_infos(hb_buf, &mut glyph_info_count);
                assert_eq!(glyph_info_count, glyph_pos_count);

                // Transform harfbuzz's glyph info into the rusty format, and add them to the
                // arena.
                let mut cursor = Point2::new(0, 0);
                let glyphs: &[GlyphData];
                {
                    let glyph_info_iter = (0..glyph_pos_count as isize).map(|i| {
                        let pos = *glyph_pos_ptr.offset(i);
                        let info = *glyph_info_ptr.offset(i);

                        let data = GlyphData {
                            pos: cursor + Vector2::new(pos.x_offset, pos.y_offset),
                            glyph_index: info.codepoint,
                            str_index: info.cluster as usize
                        };
                        cursor += Vector2::new(pos.x_advance, pos.y_advance) / 64;
                        data
                    });

                    glyphs = shaper.glyph_arena.alloc_extend(glyph_info_iter);
                }

                Some(Word {
                    advance: cursor.x,
                    text: word,
                    glyphs,
                    hard_break
                })
            }
            None => None
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

impl Drop for FTLib {
    fn drop(&mut self) {
        unsafe{ ft::FT_Done_Library(self.lib) };
    }
}

impl<B> Drop for Face<B>
    where B: StableDeref + Deref<Target=[u8]>
{
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
