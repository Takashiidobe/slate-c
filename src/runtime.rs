use cranelift_jit::JITBuilder;
use std::ffi::c_void;

pub const ARCHIVE: &str = env!("SLATE_F80_ARCHIVE");

macro_rules! helpers {
    ($(fn $name:ident($($argument:ident: $ty:ty),*) $(-> $result:ty)?;)*) => {
        unsafe extern "C" {
            $(fn $name($($argument: $ty),*) $(-> $result)?;)*
        }

        pub(crate) fn register(builder: &mut JITBuilder) {
            $(builder.symbol(stringify!($name), $name as *const u8);)*
        }
    };
}

helpers! {
    fn __slate_f80_add(out: *mut c_void, left: *const c_void, right: *const c_void);
    fn __slate_f80_sub(out: *mut c_void, left: *const c_void, right: *const c_void);
    fn __slate_f80_mul(out: *mut c_void, left: *const c_void, right: *const c_void);
    fn __slate_f80_div(out: *mut c_void, left: *const c_void, right: *const c_void);
    fn __slate_f80_sqrt(out: *mut c_void, source: *const c_void);
    fn __slate_f80_neg(out: *mut c_void, source: *const c_void);
    fn __slate_f80_eq(left: *const c_void, right: *const c_void) -> i32;
    fn __slate_f80_lt(left: *const c_void, right: *const c_void) -> i32;
    fn __slate_f80_le(left: *const c_void, right: *const c_void) -> i32;
    fn __slate_f80_from_i64(out: *mut c_void, value: i64);
    fn __slate_f80_from_u64(out: *mut c_void, value: u64);
    fn __slate_f80_from_f32(out: *mut c_void, value: f32);
    fn __slate_f80_from_f64(out: *mut c_void, value: f64);
    fn __slate_f80_to_i64(source: *const c_void) -> i64;
    fn __slate_f80_to_u64(source: *const c_void) -> u64;
    fn __slate_f80_to_f32(source: *const c_void) -> f32;
    fn __slate_f80_to_f64(source: *const c_void) -> f64;
    fn __slate_f80_set_rounding(mode: u32) -> i32;
    fn __slate_f80_get_flags() -> u32;
    fn __slate_f80_clear_flags();
}
