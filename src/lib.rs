extern crate harfbuzz_sys;
extern crate freetype;
extern crate libc;
#[macro_use]
extern crate lazy_static;
extern crate owning_ref;
extern crate unicode_segmentation;
extern crate unicode_bidi;
extern crate unicode_script;
extern crate cgmath;

mod hb_funcs;

use libc::{c_int, c_uint, c_char};
use freetype::freetype as ft;
use ft::{FT_Face, FT_Library, FT_Error, FT_Size_RequestRec_, FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL};

use harfbuzz_sys::*;

use owning_ref::StableAddress;

use std::{mem, ptr, slice};
use std::cell::Cell;
use std::ops::Deref;

use cgmath::Point2;

use unicode_segmentation::{UWordBounds, UnicodeSegmentation};


#[derive(Debug)]
pub struct FTLib {
    lib: FT_Library
}

pub struct Face<B>
    where B: StableAddress + Deref<Target=[u8]>
{
    font_buffer: B,
    ft_face: FT_Face,
    ft_size_request: FT_Size_RequestRec_,
    hb_font: *mut hb_font_t,
    _lib: FTLib
}

pub struct Shaper {
    hb_buf: *mut hb_buffer_t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontSize {
    width: u32,
    height: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DPI {
    hori: u32,
    vert: u32
}

/// TODO: HANDLE BIDI TEXT
pub struct WordIter<'a> {
    unicode_words: UWordBounds<'a>,
    text: &'a str,
    // Borrowed from a `Shaper`
    hb_buf: *mut hb_buffer_t,
    // Borrowed from a `Face`
    ft_face: FT_Face
}

pub struct Word<'a> {
    /// The advance of the word. If this is 0, the advance hasn't been computed yet.
    advance: Cell<i32>,
    text: &'a str,
    glyph_positions: &'a [hb_glyph_position_t],
    glyph_infos: &'a [hb_glyph_info_t]
}


impl FTLib {
    pub fn new() -> FTLib {
        let mut lib = ptr::null_mut();
        unsafe {
            assert_eq!(FT_Error(0), ft::FT_New_Library(ptr::null_mut(), &mut lib));
            ft::FT_Add_Default_Modules(lib);
        }

        FTLib{ lib }
    }
}

impl<B> Face<B>
    where B: StableAddress + Deref<Target=[u8]>
{
    pub fn new(font_buffer: B, face_index: i32, lib: &FTLib) -> Result<Face<B>, Error> {
        let mut ft_face = ptr::null_mut();
        unsafe {
            let err_raw = ft::FT_New_Memory_Face( lib.lib, font_buffer.as_ptr(), font_buffer.len() as c_int, face_index, &mut ft_face);

            match Error::from_raw(err_raw).unwrap() {
                Error::Ok => {
                    let hb_blob = hb_blob_create(font_buffer.as_ptr() as *const c_char, font_buffer.len() as c_uint, HB_MEMORY_MODE_READONLY, ptr::null_mut(), None);

                    let hb_face = hb_face_create(hb_blob, face_index as c_uint);
                    hb_face_set_upem(hb_face, (*ft_face).units_per_EM as c_uint);

                    let hb_font = hb_font_create(hb_face);
                    hb_funcs::set_for_font(hb_font, ft_face);

                    hb_blob_destroy(hb_blob);
                    hb_face_destroy(hb_face);


                    Ok(Face {
                        font_buffer,
                        ft_face,
                        hb_font,
                        ft_size_request: FT_Size_RequestRec_ {
                            type_: FT_Size_Request_Type__FT_SIZE_REQUEST_TYPE_NOMINAL,
                            width: -1,
                            height: -1,
                            horiResolution: 0,
                            vertResolution: 0
                        },

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
                hb_buf: hb_buffer_create()
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
        where B: StableAddress + Deref<Target=[u8]>
    {
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

        unsafe {
            // hb_buffer_add_utf8(self.hb_buf, text.as_ptr() as *const c_char, text.len() as i32, 0, text.len() as i32);
            // hb_buffer_guess_segment_properties(self.hb_buf);

            // hb_shape(face.hb_font, self.hb_buf, ptr::null(), 0);

            // let (mut glyph_info_count, mut glyph_pos_count) = (0, 0);
            // let glyph_pos_ptr = hb_buffer_get_glyph_infos(self.hb_buf, &mut glyph_pos_count);
            // let glyph_info_ptr = hb_buffer_get_glyph_infos(self.hb_buf, &mut glyph_info_count);
            // assert_eq!(glyph_info_count, glyph_pos_count);

            // let glyph_positions = slice::from_raw_parts(glyph_pos_ptr, glyph_pos_count as usize);
            // let glyph_infos = slice::from_raw_parts(glyph_info_ptr, glyph_info_count as usize);

            Ok(WordIter {
                unicode_words: text.split_word_bounds(),
                text,
                hb_buf: self.hb_buf,
                ft_face: face.ft_face
            })
        }
    }
}

impl<'a> Word<'a> {
    pub fn advance(&self) -> i32 {
        match self.advance.get() {
            0 => {
                let advance = self.glyph_positions.iter().fold(0, |adv, p| adv + p.x_advance);
                self.advance.set(advance);
                advance
            },
            _ => self.advance.get()
        }
    }

    pub fn is_whitespace(&self) -> bool {
        // If this is whitspace, then by definition no glyphs are available to draw
        self.glyph_positions.len() == 0
    }
}

// impl<'a> Iterator for WordIter<'a> {
//     type Item = Word<'a>;

//     #[inline(always)]
//     fn next(&mut self) -> Option<Word<'a>> {
//         match self.unicode_words.next() {
//             Some(word) => {

//             }
//             None => None
//         }
//     }
// }

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
    where B: StableAddress + Deref<Target=[u8]>
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
    Ok = ft::FT_Err_Ok as isize,
    CannotOpenResource = ft::FT_Err_Cannot_Open_Resource as isize,
    UnknownFileFormat = ft::FT_Err_Unknown_File_Format as isize,
    InvalidFileFormat = ft::FT_Err_Invalid_File_Format as isize,
    InvalidVersion = ft::FT_Err_Invalid_Version as isize,
    LowerModuleVersion = ft::FT_Err_Lower_Module_Version as isize,
    InvalidArgument = ft::FT_Err_Invalid_Argument as isize,
    UnimplementedFeature = ft::FT_Err_Unimplemented_Feature as isize,
    InvalidTable = ft::FT_Err_Invalid_Table as isize,
    InvalidOffset = ft::FT_Err_Invalid_Offset as isize,
    ArrayTooLarge = ft::FT_Err_Array_Too_Large as isize,
    MissingModule = ft::FT_Err_Missing_Module as isize,
    MissingProperty = ft::FT_Err_Missing_Property as isize,
    InvalidGlyphIndex = ft::FT_Err_Invalid_Glyph_Index as isize,
    InvalidCharacterCode = ft::FT_Err_Invalid_Character_Code as isize,
    InvalidGlyphFormat = ft::FT_Err_Invalid_Glyph_Format as isize,
    CannotRenderGlyph = ft::FT_Err_Cannot_Render_Glyph as isize,
    InvalidOutline = ft::FT_Err_Invalid_Outline as isize,
    InvalidComposite = ft::FT_Err_Invalid_Composite as isize,
    TooManyHints = ft::FT_Err_Too_Many_Hints as isize,
    InvalidPixelSize = ft::FT_Err_Invalid_Pixel_Size as isize,
    InvalidHandle = ft::FT_Err_Invalid_Handle as isize,
    InvalidLibraryHandle = ft::FT_Err_Invalid_Library_Handle as isize,
    InvalidDriverHandle = ft::FT_Err_Invalid_Driver_Handle as isize,
    InvalidFaceHandle = ft::FT_Err_Invalid_Face_Handle as isize,
    InvalidSizeHandle = ft::FT_Err_Invalid_Size_Handle as isize,
    InvalidSlotHandle = ft::FT_Err_Invalid_Slot_Handle as isize,
    InvalidCharMapHandle = ft::FT_Err_Invalid_CharMap_Handle as isize,
    InvalidCacheHandle = ft::FT_Err_Invalid_Cache_Handle as isize,
    InvalidStreamHandle = ft::FT_Err_Invalid_Stream_Handle as isize,
    TooManyDrivers = ft::FT_Err_Too_Many_Drivers as isize,
    TooManyExtensions = ft::FT_Err_Too_Many_Extensions as isize,
    OutOfMemory = ft::FT_Err_Out_Of_Memory as isize,
    UnlistedObject = ft::FT_Err_Unlisted_Object as isize,
    CannotOpenStream = ft::FT_Err_Cannot_Open_Stream as isize,
    InvalidStreamSeek = ft::FT_Err_Invalid_Stream_Seek as isize,
    InvalidStreamSkip = ft::FT_Err_Invalid_Stream_Skip as isize,
    InvalidStreamRead = ft::FT_Err_Invalid_Stream_Read as isize,
    InvalidStreamOperation = ft::FT_Err_Invalid_Stream_Operation as isize,
    InvalidFrameOperation = ft::FT_Err_Invalid_Frame_Operation as isize,
    NestedFrameAccess = ft::FT_Err_Nested_Frame_Access as isize,
    InvalidFrameRead = ft::FT_Err_Invalid_Frame_Read as isize,
    RasterUninitialized = ft::FT_Err_Raster_Uninitialized as isize,
    RasterCorrupted = ft::FT_Err_Raster_Corrupted as isize,
    RasterOverflow = ft::FT_Err_Raster_Overflow as isize,
    RasterNegativeHeight = ft::FT_Err_Raster_Negative_Height as isize,
    TooManyCaches = ft::FT_Err_Too_Many_Caches as isize,
    InvalidOpcode = ft::FT_Err_Invalid_Opcode as isize,
    TooFewArguments = ft::FT_Err_Too_Few_Arguments as isize,
    StackOverflow = ft::FT_Err_Stack_Overflow as isize,
    CodeOverflow = ft::FT_Err_Code_Overflow as isize,
    BadArgument = ft::FT_Err_Bad_Argument as isize,
    DivideByZero = ft::FT_Err_Divide_By_Zero as isize,
    InvalidReference = ft::FT_Err_Invalid_Reference as isize,
    DebugOpCode = ft::FT_Err_Debug_OpCode as isize,
    ENDFInExecStream = ft::FT_Err_ENDF_In_Exec_Stream as isize,
    NestedDEFS = ft::FT_Err_Nested_DEFS as isize,
    InvalidCodeRange = ft::FT_Err_Invalid_CodeRange as isize,
    ExecutionTooLong = ft::FT_Err_Execution_Too_Long as isize,
    TooManyFunctionDefs = ft::FT_Err_Too_Many_Function_Defs as isize,
    TooManyInstructionDefs = ft::FT_Err_Too_Many_Instruction_Defs as isize,
    TableMissing = ft::FT_Err_Table_Missing as isize,
    HorizHeaderMissing = ft::FT_Err_Horiz_Header_Missing as isize,
    LocationsMissing = ft::FT_Err_Locations_Missing as isize,
    NameTableMissing = ft::FT_Err_Name_Table_Missing as isize,
    CMapTableMissing = ft::FT_Err_CMap_Table_Missing as isize,
    HmtxTableMissing = ft::FT_Err_Hmtx_Table_Missing as isize,
    PostTableMissing = ft::FT_Err_Post_Table_Missing as isize,
    InvalidHorizMetrics = ft::FT_Err_Invalid_Horiz_Metrics as isize,
    InvalidCharMapFormat = ft::FT_Err_Invalid_CharMap_Format as isize,
    InvalidPPem = ft::FT_Err_Invalid_PPem as isize,
    InvalidVertMetrics = ft::FT_Err_Invalid_Vert_Metrics as isize,
    CouldNotFindContext = ft::FT_Err_Could_Not_Find_Context as isize,
    InvalidPostTableFormat = ft::FT_Err_Invalid_Post_Table_Format as isize,
    InvalidPostTable = ft::FT_Err_Invalid_Post_Table as isize,
    DEFInGlyfBytecode = ft::FT_Err_DEF_In_Glyf_Bytecode as isize,
    MissingBitmap = ft::FT_Err_Missing_Bitmap as isize,
    SyntaxError = ft::FT_Err_Syntax_Error as isize,
    StackUnderflow = ft::FT_Err_Stack_Underflow as isize,
    Ignore = ft::FT_Err_Ignore as isize,
    NoUnicodeGlyphName = ft::FT_Err_No_Unicode_Glyph_Name as isize,
    GlyphTooBig = ft::FT_Err_Glyph_Too_Big as isize,
    MissingStartfontField = ft::FT_Err_Missing_Startfont_Field as isize,
    MissingFontField = ft::FT_Err_Missing_Font_Field as isize,
    MissingSizeField = ft::FT_Err_Missing_Size_Field as isize,
    MissingFontboundingboxField = ft::FT_Err_Missing_Fontboundingbox_Field as isize,
    MissingCharsField = ft::FT_Err_Missing_Chars_Field as isize,
    MissingStartcharField = ft::FT_Err_Missing_Startchar_Field as isize,
    MissingEncodingField = ft::FT_Err_Missing_Encoding_Field as isize,
    MissingBbxField = ft::FT_Err_Missing_Bbx_Field as isize,
    BbxTooBig = ft::FT_Err_Bbx_Too_Big as isize,
    CorruptedFontHeader = ft::FT_Err_Corrupted_Font_Header as isize,
    CorruptedFontGlyphs = ft::FT_Err_Corrupted_Font_Glyphs as isize,
    Max = ft::FT_Err_Max as isize,
}

impl Error {
    pub fn from_raw(err: FT_Error) -> Option<Error> {
        let err_in_bounds = move |left, right| left <= err.0 && err.0 <= right;

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
