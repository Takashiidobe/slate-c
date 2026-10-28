use cranelift_jit::JITBuilder;
use std::ffi::c_void;

pub const ARCHIVE: &str = env!("SLATE_F80_ARCHIVE");

macro_rules! helpers {
    ($(fn $name:ident($($argument:ident: $ty:ty),*) $(-> $result:ty)?;)*) => {
        unsafe extern "C" {
            $(fn $name($($argument: $ty),*) $(-> $result)?;)*
        }

        pub(crate) fn register(builder: &mut JITBuilder) {
            register_libraries(builder);
            $(builder.symbol(stringify!($name), $name as *const u8);)*
        }
    };
}

macro_rules! libraries {
    ($(fn $name:ident($($argument:ident: $ty:ty),*) -> $result:ty;)*) => {
        #[link(name = "m")]
        #[link(name = "gcc")]
        unsafe extern "C" {
            $(fn $name($($argument: $ty),*) -> $result;)*
        }

        fn register_libraries(builder: &mut JITBuilder) {
            $(builder.symbol(stringify!($name), $name as *const u8);)*
        }
    };
}

libraries! {
    fn memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, value: i32, size: usize) -> *mut c_void;
    fn memcmp(left: *const c_void, right: *const c_void, size: usize) -> i32;
    fn sqrt(value: f64) -> f64;
    fn sqrtf(value: f32) -> f32;
    fn fma(a: f64, b: f64, c: f64) -> f64;
    fn fmaf(a: f32, b: f32, c: f32) -> f32;
    fn sin(value: f64) -> f64;
    fn cos(value: f64) -> f64;
    fn __popcountdi2(value: u64) -> i32;
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
