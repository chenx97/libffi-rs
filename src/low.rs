//! A low-level wrapping of libffi. This layer makes no attempts at safety,
//! but tries to provide a somewhat more idiomatic interface. It also
//! re-exports types and constants necessary for using the library.

use c;

use std::mem;
use std::os::raw::{c_void, c_uint};

/// The two kinds of errors reported by libffi.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Error { BadTypedef, BadAbi }

/// A specialized `Result` type for libffi operations.
pub type Result<T> = ::std::result::Result<T, Error>;

fn ffi_status_to_result<R>(status: c::ffi_status, good: R) -> Result<R> {
    use c::ffi_status::*;
    match status {
        FFI_OK => Ok(good),
        FFI_BAD_TYPEDEF => Err(Error::BadTypedef),
        FFI_BAD_ABI => Err(Error::BadAbi),
    }
}

pub use c::ffi_abi;
pub use c::_ffi_type as ffi_type;
pub use c::ffi_status;

pub use c::ffi_cif;
pub use c::ffi_closure;

pub use c::ffi_type_void;
pub use c::ffi_type_uint8;
pub use c::ffi_type_sint8;
pub use c::ffi_type_uint16;
pub use c::ffi_type_sint16;
pub use c::ffi_type_uint32;
pub use c::ffi_type_sint32;
pub use c::ffi_type_uint64;
pub use c::ffi_type_sint64;
pub use c::ffi_type_float;
pub use c::ffi_type_double;
pub use c::ffi_type_pointer;
pub use c::ffi_type_longdouble;
pub use c::ffi_type_complex_float;
pub use c::ffi_type_complex_double;
pub use c::ffi_type_complex_longdouble;

/// Initalizes a CIF (Call InterFace) with the given ABI and types.
/// Note that the CIF retains references to `rtype` and `atypes`, so if
/// they are no longer live when the CIF is used then the result is
/// undefined.
pub unsafe fn prep_cif(cif: *mut ffi_cif,
                       abi: ffi_abi,
                       nargs: usize,
                       rtype: *mut ffi_type,
                       atypes: *mut *mut ffi_type) -> Result<()>
{
    let status = c::ffi_prep_cif(cif, abi,
                                 nargs as c_uint,
                                 rtype, atypes);
    ffi_status_to_result(status, ())
}

/// Initalizes a CIF (Call InterFace) for a varargs function with
/// the given ABI and types.
pub unsafe fn prep_cif_var(cif: *mut ffi_cif,
                           abi: ffi_abi,
                           nfixedargs: usize,
                           ntotalargs: usize,
                           rtype: *mut ffi_type,
                           atypes: *mut *mut ffi_type) -> Result<()>
{
    let status = c::ffi_prep_cif_var(cif, abi,
                                     nfixedargs as c_uint,
                                     ntotalargs as c_uint,
                                     rtype, atypes);
    ffi_status_to_result(status, ())
}

/// Calls a C function using the calling convention and types specified
/// by the given CIF.
pub unsafe fn call<R>(cif:  *mut ffi_cif,
                      fun:  extern "C" fn(),
                      args: *mut *mut c_void) -> R
{
    let mut result: R = mem::uninitialized();
    c::ffi_call(cif, Some(fun), mem::transmute(&mut result as *mut R), args);
    result
}

/// Allocates a closure, returning a pair of the writable closure
/// object and the function pointer for calling it.
pub fn closure_alloc() -> (*mut ffi_closure, extern "C" fn()) {
    unsafe {
        let mut code_pointer: *mut c_void = mem::uninitialized();
        let closure = c::ffi_closure_alloc(mem::size_of::<ffi_closure>(),
                                           &mut code_pointer);
        (mem::transmute(closure), mem::transmute(code_pointer))
    }
}

/// Frees the resources associated with a closure.
pub unsafe fn closure_free(closure: *mut ffi_closure) {
    c::ffi_closure_free(mem::transmute(closure));
}

/// The type of function called by a closure. `U` is the type of
/// the user data captured by the closure and passed to the callback.
pub type Callback<U>
    = unsafe extern "C" fn(cif:      *mut ffi_cif,
                           result:   *mut c_void,
                           args:     *mut *mut c_void,
                           userdata: *mut U);

/// Prepares a closure to call the given callback function with the
/// given user data.
pub unsafe fn prep_closure_loc<U>(closure:  *mut ffi_closure,
                                  cif:      *mut ffi_cif,
                                  callback: Callback<U>,
                                  userdata: *mut U,
                                  code:     extern "C" fn()) -> Result<()>
{
    let status = c::ffi_prep_closure_loc(closure,
                                         cif,
                                         Some(mem::transmute(callback)),
                                         mem::transmute(userdata),
                                         mem::transmute(code));
    ffi_status_to_result(status, ())
}

#[cfg(test)]
mod test {
    use c;
    use super::*;
    use std::mem;
    use std::os::raw::c_void;

    unsafe extern "C" fn callback(_cif: *mut ffi_cif,
                                  result: *mut c_void,
                                  args: *mut *mut c_void,
                                  userdata: *mut c_void)
    {
        let result:    *mut u64 = mem::transmute(result);
        let args: *mut *mut u64 = mem::transmute(args);
        let userdata:  *mut u64 = mem::transmute(userdata);
        *result = **args + *userdata;
    }

    #[test]
    fn closure() {
        unsafe {
            let mut cif: ffi_cif = Default::default();
            let mut args: [*mut ffi_type; 1] = [&mut ffi_type_uint64];
            let mut env: u64 = 5;

            prep_cif(&mut cif, c::FFI_DEFAULT_ABI, 1, &mut ffi_type_uint64,
                     args.as_mut_ptr()).unwrap();

            let (closure, fun_) = closure_alloc();
            let fun: unsafe extern "C" fn(u64) -> u64 = mem::transmute(fun_);

            prep_closure_loc(closure,
                             &mut cif,
                             callback,
                             mem::transmute(&mut env),
                             mem::transmute(fun)).unwrap();

            assert_eq!(11, fun(6));
            assert_eq!(12, fun(7));
        }
    }
}
