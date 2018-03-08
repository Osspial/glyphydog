use ft::*;
use harfbuzz_sys::*;

use libc::{c_void, c_uint, c_int};
use std::{ptr, mem};

pub unsafe fn set_for_font(hb_font: *mut hb_font_t, ft_face: FT_Face) {
    hb_font_set_funcs(
        hb_font,
        HB_FUNCS.0,
        Box::into_raw(Box::new(FontFuncData {
            ft_face,
            load_flags: (FT_LOAD_DEFAULT | FT_LOAD_NO_HINTING) as c_int,
            symbol: (*ft_face).charmap != ptr::null_mut() && (*(*ft_face).charmap).encoding == FT_Encoding__FT_ENCODING_MS_SYMBOL
        })) as *mut c_void,
        Some(drop_font_func_data)
    );
}

struct FontFuncsWrapper(pub *mut hb_font_funcs_t);
unsafe impl ::std::marker::Sync for FontFuncsWrapper {}
impl Drop for FontFuncsWrapper {
    fn drop(&mut self) {
        unsafe{ hb_font_funcs_destroy(self.0) };
    }
}

lazy_static!{
    static ref HB_FUNCS: FontFuncsWrapper = unsafe {
        let funcs = hb_font_funcs_create();
        hb_font_funcs_set_font_h_extents_func(funcs, Some(get_font_h_extents), ptr::null_mut(), None);
        hb_font_funcs_set_nominal_glyph_func(funcs, Some(get_font_nominal_glyph), ptr::null_mut(), None);
        hb_font_funcs_set_variation_glyph_func(funcs, Some(get_variation_glyph), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_h_advance_func(funcs, Some(get_h_advance), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_v_advance_func(funcs, Some(get_v_advance), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_v_origin_func(funcs, Some(get_v_origin), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_h_kerning_func(funcs, Some(get_h_kerning), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_extents_func(funcs, Some(get_extents), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_contour_point_func(funcs, Some(get_contour_point), ptr::null_mut(), None);
        hb_font_funcs_set_glyph_name_func(funcs, Some(get_glyph_name), ptr::null_mut(), None);
        // hb_font_funcs_set_glyph_from_name_func(funcs, None, ptr::null_mut(), None);

        hb_font_funcs_make_immutable(funcs);
        FontFuncsWrapper(funcs)
    };
}

struct FontFuncData {
    ft_face: FT_Face,
    load_flags: c_int,
    symbol: bool
}

unsafe extern "C" fn drop_font_func_data(ffd: *mut c_void) {
    Box::from_raw(ffd as *mut FontFuncData);
}

// These functions are pretty much a direct Rust translation of hb-ft.cc's functions

unsafe extern "C" fn get_font_h_extents(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    metrics: *mut hb_font_extents_t,
    _: *mut c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);

    let ft_metrics = &(*(*ffd.ft_face).size).metrics;
    let hb_metrics = &mut *metrics;
    hb_metrics.ascender = ft_metrics.ascender as hb_position_t;
    hb_metrics.descender = ft_metrics.descender as hb_position_t;
    hb_metrics.line_gap = (ft_metrics.height - (ft_metrics.ascender - ft_metrics.descender)) as hb_position_t;

    if ft_metrics.y_scale < 0 {
        hb_metrics.ascender *= -1;
        hb_metrics.descender *= -1;
        hb_metrics.line_gap *= -1;
    }

    1
}

unsafe extern "C" fn get_font_nominal_glyph(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    unicode: hb_codepoint_t,
    glyph: *mut hb_codepoint_t,
    _: *mut c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);
    let mut char_index = FT_Get_Char_Index(ffd.ft_face, unicode as FT_ULong);

    if char_index == 0 && ffd.symbol && unicode <= 0x00FF {
        char_index = FT_Get_Char_Index(ffd.ft_face, 0xF000 + unicode as FT_ULong);
        if char_index == 0 {
            return 0;
        }
    }

    *glyph = char_index;

    1
}

unsafe extern "C" fn get_variation_glyph(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    unicode: hb_codepoint_t,
    variation_selector: hb_codepoint_t,
    glyph: *mut hb_codepoint_t, _: *mut c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);
    let char_index = FT_Face_GetCharVariantIndex(ffd.ft_face, unicode as FT_ULong, variation_selector);

    match char_index {
        0 => 0,
        _ => {
            *glyph = char_index;
            1
        }
    }
}

unsafe extern "C" fn get_h_advance(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    glyph: hb_codepoint_t,
    _: *mut c_void
) -> hb_position_t
{
    let ffd = &*(font_data as *const FontFuncData);

    let mut advance = 0;
    match FT_Get_Advance(ffd.ft_face, glyph, ffd.load_flags, &mut advance) {
        FT_Error(0) => {
            if (*(*ffd.ft_face).size).metrics.x_scale < 0 {
                advance *= -1;
            }

            ((advance + (1<<9)) >> 10) as hb_position_t
        },
        _ => 0
    }
}

unsafe extern "C" fn get_v_advance(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    glyph: hb_codepoint_t,
    _: *mut c_void
) -> hb_position_t {
    let ffd = &*(font_data as *const FontFuncData);

    let mut advance = 0;
    match FT_Get_Advance(ffd.ft_face, glyph, ffd.load_flags | FT_LOAD_VERTICAL_LAYOUT as c_int, &mut advance) {
        FT_Error(0) => {
            if (*(*ffd.ft_face).size).metrics.y_scale < 0 {
                advance *= -1;
            }

            ((-advance + (1<<9)) >> 10) as hb_position_t
        },
        _ => 0
    }
}

unsafe extern "C" fn get_v_origin(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    glyph: hb_codepoint_t,
    x: *mut hb_position_t,
    y: *mut hb_position_t,
    _: *mut c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);

    match FT_Load_Glyph(ffd.ft_face, glyph, ffd.load_flags) {
        FT_Error(0) => {
            let glyph_metrics = (*(*ffd.ft_face).glyph).metrics;
            *x = (glyph_metrics.horiBearingX - glyph_metrics.vertBearingX) as hb_position_t;
            *y = (glyph_metrics.horiBearingY - (-glyph_metrics.vertBearingY)) as hb_position_t;

            if (*(*ffd.ft_face).size).metrics.x_scale < 0 {
                *x *= -1;
            }
            if (*(*ffd.ft_face).size).metrics.y_scale < 0 {
                *y *= -1;
            }

            1
        },
        _ => 0
    }
}

unsafe extern "C" fn get_h_kerning(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    left_glyph: hb_codepoint_t,
    right_glyph: hb_codepoint_t,
    _: *mut c_void
) -> hb_position_t
{
    let ffd = &*(font_data as *const FontFuncData);

    let mut kerningv = mem::uninitialized();
    let mode = match (*(*ffd.ft_face).size).metrics.x_ppem {
        0 => FT_Kerning_Mode__FT_KERNING_UNFITTED,
        _ => FT_Kerning_Mode__FT_KERNING_DEFAULT
    };
    match FT_Get_Kerning(ffd.ft_face, left_glyph, right_glyph, mode as c_uint, &mut kerningv) {
        FT_Error(0) => kerningv.x as hb_position_t,
        _ => 0
    }
}

unsafe extern "C" fn get_extents(
    _: *mut hb_font_t,
    font_data: *mut c_void,
    glyph: hb_codepoint_t,
    extents: *mut hb_glyph_extents_t,
    _: *mut c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);
    let extents = &mut *extents;

    match FT_Load_Glyph(ffd.ft_face, glyph, ffd.load_flags) {
        FT_Error(0) => {
            let glyph_metrics = (*(*ffd.ft_face).glyph).metrics;
            extents.x_bearing = glyph_metrics.horiBearingX as hb_position_t;
            extents.y_bearing = glyph_metrics.horiBearingY as hb_position_t;
            extents.width = glyph_metrics.width as hb_position_t;
            extents.height = glyph_metrics.height as hb_position_t;

            if (*(*ffd.ft_face).size).metrics.x_scale < 0 {
                extents.x_bearing *= -1;
                extents.width *= -1;
            }
            if (*(*ffd.ft_face).size).metrics.y_scale < 0 {
                extents.y_bearing *= -1;
                extents.height *= -1;
            }

            1
        },
        _ => 0
    }
}

unsafe extern "C" fn get_contour_point(
    _: *mut hb_font_t,
    font_data: *mut ::libc::c_void,
    glyph: hb_codepoint_t,
    point_index: ::libc::c_uint,
    x: *mut hb_position_t,
    y: *mut hb_position_t,
    _: *mut ::libc::c_void
) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);

    if FT_Error(0) != FT_Load_Glyph(ffd.ft_face, glyph, ffd.load_flags) {
        return 0;
    }

    let glyph = &*(*ffd.ft_face).glyph;
    if glyph.format != FT_Glyph_Format__FT_GLYPH_FORMAT_OUTLINE {
        return 0;
    }
    if point_index as i64 >= glyph.outline.n_points as i64 {
        return 0;
    }

    *x = (*glyph.outline.points.offset(point_index as isize)).x as hb_position_t;
    *y = (*glyph.outline.points.offset(point_index as isize)).y as hb_position_t;

    1
}

unsafe extern "C" fn get_glyph_name(
    _: *mut hb_font_t,
    font_data: *mut ::libc::c_void,
    glyph: hb_codepoint_t,
    name: *mut ::libc::c_char,
    size: ::libc::c_uint,
    _: *mut ::libc::c_void
    ) -> hb_bool_t
{
    let ffd = &*(font_data as *const FontFuncData);

    let mut ret = FT_Error(0) == FT_Get_Glyph_Name(ffd.ft_face, glyph, name as *mut _, size);
    if ret && (size != 0 && *name != 0) {
        ret = false;
    }
    ret as hb_bool_t
}
