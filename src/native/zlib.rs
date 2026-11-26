use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
use std::sync::OnceLock;

use super::bridge::{register_handler, register_handler_in, CallContext};
use super::call_assign::CallBuilder;

const LIBZ: &str = "libz.so";
const Z_VERSION_ERROR: c_int = -6;
const ALLOCATOR_OFFSET: usize = 8 * std::mem::size_of::<u64>();

#[link(name = "z")]
extern "C" {
    fn adler32(adler: c_ulong, buffer: *const u8, length: c_uint) -> c_ulong;
    fn deflateInit2_(
        stream: *mut c_void,
        level: c_int,
        method: c_int,
        window_bits: c_int,
        memory_level: c_int,
        strategy: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    fn deflate(stream: *mut c_void, flush: c_int) -> c_int;
    fn deflateBound(stream: *mut c_void, source_length: c_ulong) -> c_ulong;
    fn deflateEnd(stream: *mut c_void) -> c_int;
    fn inflateInit_(stream: *mut c_void, version: *const c_char, stream_size: c_int) -> c_int;
    fn inflate(stream: *mut c_void, flush: c_int) -> c_int;
    fn inflateEnd(stream: *mut c_void) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Allocator {
    zalloc: usize,
    zfree: usize,
    opaque: usize,
}

struct GuestAllocator {
    guest: Allocator,
    failure: Option<String>,
}

impl GuestAllocator {
    fn call(&mut self, call: CallBuilder) -> usize {
        match call.invoke() {
            Ok(result) => result.u64() as usize,
            Err(error) => {
                self.failure.get_or_insert_with(|| error.to_string());
                0
            }
        }
    }
}

unsafe extern "C" fn allocate(opaque: *mut c_void, items: c_uint, size: c_uint) -> *mut c_void {
    let allocator = unsafe { &mut *opaque.cast::<GuestAllocator>() };
    let call = CallBuilder::new(allocator.guest.zalloc)
        .ptr(allocator.guest.opaque)
        .u32(items)
        .u32(size);
    allocator.call(call) as *mut c_void
}

unsafe extern "C" fn release(opaque: *mut c_void, address: *mut c_void) {
    let allocator = unsafe { &mut *opaque.cast::<GuestAllocator>() };
    let call = CallBuilder::new(allocator.guest.zfree)
        .ptr(allocator.guest.opaque)
        .ptr(address as usize);
    allocator.call(call);
}

fn allocator_of(stream: usize) -> *mut Allocator {
    (stream + ALLOCATOR_OFFSET) as *mut Allocator
}

fn with_guest_allocator(
    context: &mut CallContext,
    stream: usize,
    operation: impl FnOnce(*mut c_void) -> c_int,
) {
    // cite: libcore zip_util.c#tracking_zlib_alloc, zlib_block_alloc
    if stream == 0 {
        context.ret_i32(operation(std::ptr::null_mut()));
        return;
    }
    let slot = allocator_of(stream);
    let guest = unsafe { slot.read_unaligned() };
    let mut allocator = GuestAllocator {
        guest,
        failure: None,
    };
    let host = Allocator {
        zalloc: if guest.zalloc == 0 {
            0
        } else {
            allocate as *const () as usize
        },
        zfree: if guest.zfree == 0 {
            0
        } else {
            release as *const () as usize
        },
        opaque: std::ptr::addr_of_mut!(allocator) as usize,
    };
    unsafe { slot.write_unaligned(host) };
    let result = operation(stream as *mut c_void);
    unsafe { slot.write_unaligned(guest) };
    match allocator.failure {
        Some(message) => context.fail(message),
        None => context.ret_i32(result),
    }
}

fn default_allocator() -> (usize, usize) {
    static DEFAULT: OnceLock<(usize, usize)> = OnceLock::new();
    *DEFAULT.get_or_init(|| {
        let zalloc = register_handler("zlib default zalloc", |context| {
            let (items, size) = (context.arg_u32(1), context.arg_u32(2));
            let block = unsafe { libc::calloc(items as usize, size as usize) };
            context.ret_ptr(block as usize);
        });
        let zfree = register_handler("zlib default zfree", |context| unsafe {
            libc::free(context.arg_ptr(1) as *mut c_void)
        });
        (zalloc, zfree)
    })
}

fn install_default_allocator(stream: usize) {
    // cite: art profile_compilation_info.cc#DeflateBuffer, InflateBuffer
    let (zalloc, zfree) = default_allocator();
    let slot = allocator_of(stream);
    let mut allocator = unsafe { slot.read_unaligned() };
    if allocator.zalloc == 0 {
        allocator.zalloc = zalloc;
        allocator.opaque = 0;
    }
    if allocator.zfree == 0 {
        allocator.zfree = zfree;
    }
    unsafe { slot.write_unaligned(allocator) };
}

fn initialize(context: &mut CallContext, stream: usize, operation: impl Fn(*mut c_void) -> c_int) {
    // cite: libcore Deflater.c#Deflater_init, Z_VERSION_ERROR
    // cite: libcore Inflater.c#Inflater_init, Z_VERSION_ERROR
    let refusal = operation(std::ptr::null_mut());
    if stream == 0 || refusal == Z_VERSION_ERROR {
        context.ret_i32(refusal);
        return;
    }
    install_default_allocator(stream);
    with_guest_allocator(context, stream, operation);
}

fn guest_deflate_init2(context: &mut CallContext) {
    // cite: libcore zip_util.c#deflateInit2Wrapper
    let stream = context.arg_ptr(0);
    let level = context.arg_i32(1);
    let method = context.arg_i32(2);
    let window_bits = context.arg_i32(3);
    let memory_level = context.arg_i32(4);
    let strategy = context.arg_i32(5);
    let version = context.arg_ptr(6) as *const c_char;
    let stream_size = context.arg_i32(7);
    initialize(context, stream, |stream| unsafe {
        deflateInit2_(
            stream,
            level,
            method,
            window_bits,
            memory_level,
            strategy,
            version,
            stream_size,
        )
    });
}

fn guest_inflate_init(context: &mut CallContext) {
    let stream = context.arg_ptr(0);
    let version = context.arg_ptr(1) as *const c_char;
    let stream_size = context.arg_i32(2);
    initialize(context, stream, |stream| unsafe {
        inflateInit_(stream, version, stream_size)
    });
}

fn guest_deflate(context: &mut CallContext) {
    let (stream, flush) = (context.arg_ptr(0), context.arg_i32(1));
    with_guest_allocator(context, stream, |stream| unsafe { deflate(stream, flush) });
}

fn guest_inflate(context: &mut CallContext) {
    let (stream, flush) = (context.arg_ptr(0), context.arg_i32(1));
    with_guest_allocator(context, stream, |stream| unsafe { inflate(stream, flush) });
}

fn guest_deflate_end(context: &mut CallContext) {
    let stream = context.arg_ptr(0);
    with_guest_allocator(context, stream, |stream| unsafe { deflateEnd(stream) });
}

fn guest_inflate_end(context: &mut CallContext) {
    let stream = context.arg_ptr(0);
    with_guest_allocator(context, stream, |stream| unsafe { inflateEnd(stream) });
}

fn guest_deflate_bound(context: &mut CallContext) {
    // cite: art profile_compilation_info.cc#deflateBound
    let stream = context.arg_ptr(0) as *mut c_void;
    let bound = unsafe { deflateBound(stream, context.arg_u64(1) as c_ulong) };
    context.ret_u64(bound);
}

fn guest_adler32(context: &mut CallContext) {
    // cite: libcore Adler32.c#Java_java_util_zip_Adler32_updateBytes
    let checksum = unsafe {
        adler32(
            context.arg_u64(0),
            context.arg_ptr(1) as *const u8,
            context.arg_u32(2),
        )
    };
    context.ret_u64(checksum);
}

pub(super) fn register() {
    register_handler_in(LIBZ, "adler32", guest_adler32);
    register_handler_in(LIBZ, "deflateInit2_", guest_deflate_init2);
    register_handler_in(LIBZ, "deflate", guest_deflate);
    register_handler_in(LIBZ, "deflateBound", guest_deflate_bound);
    register_handler_in(LIBZ, "deflateEnd", guest_deflate_end);
    register_handler_in(LIBZ, "inflateInit_", guest_inflate_init);
    register_handler_in(LIBZ, "inflate", guest_inflate);
    register_handler_in(LIBZ, "inflateEnd", guest_inflate_end);
}

#[cfg(test)]
mod tests {
    use std::ffi::{c_void, CStr};

    use super::CallContext;
    use crate::error::Result;
    use crate::native::{
        bridge::{register_handler, BridgeProvider},
        call_assign::CallBuilder,
        handler_registry::register_all_groups,
        link_resolution::SystemLibraryProvider,
    };

    const Z_NO_FLUSH: i32 = 0;
    const Z_FINISH: i32 = 4;
    const Z_OK: i32 = 0;
    const Z_STREAM_END: i32 = 1;
    const Z_STREAM_ERROR: i32 = -2;
    const Z_MEM_ERROR: i32 = -4;
    const Z_VERSION_ERROR: i32 = -6;
    const GZIP_WINDOW_BITS: i32 = 31;
    const ZLIB_WINDOW_BITS: i32 = 15;
    const VERSION: &CStr = c"1.3.1";

    #[repr(C)]
    #[derive(Default)]
    struct ZStream {
        next_in: usize,
        avail_in: u32,
        total_in: u64,
        next_out: usize,
        avail_out: u32,
        total_out: u64,
        msg: usize,
        state: usize,
        zalloc: usize,
        zfree: usize,
        opaque: usize,
        data_type: i32,
        adler: u64,
        reserved: u64,
    }

    const STREAM_SIZE: i32 = std::mem::size_of::<ZStream>() as i32;

    fn export(name: &str) -> CallBuilder {
        register_all_groups();
        CallBuilder::new(BridgeProvider.export("libz.so", name).unwrap())
    }

    fn address(stream: &mut ZStream) -> usize {
        stream as *mut ZStream as usize
    }

    fn try_deflate_init(
        stream: usize,
        window_bits: i32,
        version: &CStr,
        stream_size: i32,
    ) -> Result<i32> {
        export("deflateInit2_")
            .ptr(stream)
            .i32(1)
            .i32(8)
            .i32(window_bits)
            .i32(8)
            .i32(0)
            .ptr(version.as_ptr() as usize)
            .i32(stream_size)
            .invoke()
            .map(|result| result.i32())
    }

    fn deflate_init(stream: &mut ZStream, window_bits: i32) -> i32 {
        try_deflate_init(address(stream), window_bits, VERSION, STREAM_SIZE).unwrap()
    }

    fn inflate_init(stream: usize, version: &CStr, stream_size: i32) -> i32 {
        export("inflateInit_")
            .ptr(stream)
            .ptr(version.as_ptr() as usize)
            .i32(stream_size)
            .invoke()
            .unwrap()
            .i32()
    }

    fn step(name: &str, stream: &mut ZStream, flush: i32) -> i32 {
        export(name)
            .ptr(address(stream))
            .i32(flush)
            .invoke()
            .unwrap()
            .i32()
    }

    fn end(name: &str, stream: &mut ZStream) -> i32 {
        export(name).ptr(address(stream)).invoke().unwrap().i32()
    }

    fn bound(stream: &mut ZStream, length: usize) -> usize {
        export("deflateBound")
            .ptr(address(stream))
            .int(length as u64)
            .invoke()
            .unwrap()
            .u64() as usize
    }

    fn compress(stream: &mut ZStream, input: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; bound(stream, input.len())];
        stream.next_in = input.as_ptr() as usize;
        stream.avail_in = input.len() as u32;
        stream.next_out = output.as_mut_ptr() as usize;
        stream.avail_out = output.len() as u32;
        assert_eq!(step("deflate", stream, Z_FINISH), Z_STREAM_END);
        assert_eq!(stream.avail_in, 0);
        output.truncate(stream.total_out as usize);
        output
    }

    fn sample() -> Vec<u8> {
        b"winedroid zlib bridge ".repeat(4096)
    }

    fn tracking_alloc(context: &mut CallContext) {
        let needed = context.arg_ptr(0) as *mut usize;
        let (items, size) = (context.arg_u32(1) as usize, context.arg_u32(2) as usize);
        unsafe { *needed += items * size };
        context.ret_ptr(unsafe { libc::calloc(items, size) } as usize);
    }

    fn block_alloc(context: &mut CallContext) {
        let range = context.arg_ptr(0) as *mut [usize; 2];
        let needed = context.arg_u32(1) as usize * context.arg_u32(2) as usize;
        let [next, end] = unsafe { *range };
        if end - next >= needed {
            unsafe { (*range)[0] = next + needed };
            context.ret_ptr(next);
        } else {
            context.ret_ptr(0);
        }
    }

    struct Allocators {
        tracking_alloc: usize,
        tracking_free: usize,
        block_alloc: usize,
        block_free: usize,
        failing_alloc: usize,
    }

    fn allocators() -> &'static Allocators {
        static ALLOCATORS: std::sync::OnceLock<Allocators> = std::sync::OnceLock::new();
        ALLOCATORS.get_or_init(|| Allocators {
            tracking_alloc: register_handler("fx_zlib_tracking_alloc", tracking_alloc),
            tracking_free: register_handler("fx_zlib_tracking_free", |context| unsafe {
                libc::free(context.arg_ptr(1) as *mut c_void)
            }),
            block_alloc: register_handler("fx_zlib_block_alloc", block_alloc),
            block_free: register_handler("fx_zlib_block_free", |_| {}),
            failing_alloc: register_handler("fx_zlib_failing_alloc", |context| {
                context.fail("fx zlib allocator failed")
            }),
        })
    }

    fn tracked(needed: &mut usize) -> ZStream {
        register_all_groups();
        ZStream {
            zalloc: allocators().tracking_alloc,
            zfree: allocators().tracking_free,
            opaque: needed as *mut usize as usize,
            ..ZStream::default()
        }
    }

    #[test]
    fn adler32_export_initializes_and_updates_the_checksum() {
        // cite: libcore Adler32.java#reset, update
        let initial = export("adler32")
            .int(0)
            .ptr(0)
            .u32(0)
            .invoke()
            .unwrap()
            .u64();
        assert_eq!(initial, 1);

        let bytes = b"123456789";
        let checksum = export("adler32")
            .int(initial)
            .ptr(bytes.as_ptr() as usize)
            .u32(bytes.len() as u32)
            .invoke()
            .unwrap()
            .u64();
        assert_eq!(checksum, 0x091e01de);
    }

    #[test]
    fn art_profile_deflate_and_inflate_round_trip_with_null_allocators() {
        // cite: art profile_compilation_info.cc#DeflateBuffer, InflateBuffer
        let input = sample();
        let mut deflater = ZStream::default();
        assert_eq!(deflate_init(&mut deflater, ZLIB_WINDOW_BITS), Z_OK);
        assert_ne!(deflater.zalloc, 0);
        assert_ne!(deflater.zfree, 0);
        let compressed = compress(&mut deflater, &input);
        assert!(compressed.len() < input.len());
        assert_eq!(end("deflateEnd", &mut deflater), Z_OK);

        let mut output = vec![0u8; input.len()];
        let mut inflater = ZStream {
            next_in: compressed.as_ptr() as usize,
            avail_in: compressed.len() as u32,
            next_out: output.as_mut_ptr() as usize,
            avail_out: output.len() as u32,
            ..ZStream::default()
        };
        assert_eq!(
            inflate_init(address(&mut inflater), VERSION, STREAM_SIZE),
            Z_OK
        );
        assert_eq!(step("inflate", &mut inflater, Z_NO_FLUSH), Z_STREAM_END);
        assert_eq!(inflater.avail_in, 0);
        assert_eq!(inflater.avail_out, 0);
        assert_eq!(end("inflateEnd", &mut inflater), Z_OK);
        assert_eq!(output, input);
    }

    #[test]
    fn libcore_gzip_allocates_through_the_guest_functions_and_opaque() {
        // cite: libcore zip_util.c#ZIP_GZip_InitParams, ZIP_GZip_Fully, tracking_zlib_alloc, zlib_block_alloc
        let input = sample();
        let mut needed = 0usize;
        let mut sizing = tracked(&mut needed);
        let opaque = sizing.opaque;
        assert_eq!(deflate_init(&mut sizing, GZIP_WINDOW_BITS), Z_OK);
        assert_eq!(sizing.zalloc, allocators().tracking_alloc);
        assert_eq!(sizing.zfree, allocators().tracking_free);
        assert_eq!(sizing.opaque, opaque);
        assert!(bound(&mut sizing, input.len()) > 0);
        assert_eq!(end("deflateEnd", &mut sizing), Z_OK);
        assert!(needed > 0);

        let mut block = vec![0u8; needed];
        let start = block.as_mut_ptr() as usize;
        let mut range = [start, start + needed];
        let mut deflater = ZStream {
            zalloc: allocators().block_alloc,
            zfree: allocators().block_free,
            opaque: range.as_mut_ptr() as usize,
            ..ZStream::default()
        };
        assert_eq!(deflate_init(&mut deflater, GZIP_WINDOW_BITS), Z_OK);
        assert!(range[0] > start);
        let compressed = compress(&mut deflater, &input);
        assert_eq!(compressed[..2], [0x1f, 0x8b]);
        assert_eq!(end("deflateEnd", &mut deflater), Z_OK);
    }

    #[test]
    fn libcore_inflate_allocates_its_window_through_the_guest_function() {
        // cite: libcore zip_util.c#InflateFully, tracking_zlib_alloc
        let input = sample();
        let mut deflater = ZStream::default();
        register_all_groups();
        assert_eq!(deflate_init(&mut deflater, ZLIB_WINDOW_BITS), Z_OK);
        let compressed = compress(&mut deflater, &input);
        assert_eq!(end("deflateEnd", &mut deflater), Z_OK);

        let mut needed = 0usize;
        let mut inflater = tracked(&mut needed);
        assert_eq!(
            inflate_init(address(&mut inflater), VERSION, STREAM_SIZE),
            Z_OK
        );
        let after_init = needed;
        let mut output = [0u8; 100];
        inflater.next_in = compressed.as_ptr() as usize;
        inflater.avail_in = compressed.len() as u32;
        inflater.next_out = output.as_mut_ptr() as usize;
        inflater.avail_out = output.len() as u32;
        assert_eq!(step("inflate", &mut inflater, Z_NO_FLUSH), Z_OK);
        assert!(needed > after_init);
        assert_eq!(output[..], input[..100]);
        assert_eq!(end("inflateEnd", &mut inflater), Z_OK);
    }

    #[test]
    fn libcore_zip_init_reports_version_mismatch_before_touching_the_stream() {
        // cite: libcore Deflater.c#Deflater_init, Z_VERSION_ERROR
        // cite: libcore Inflater.c#Inflater_init, Z_VERSION_ERROR
        let mut stream = ZStream::default();
        let target = address(&mut stream);
        assert_eq!(
            try_deflate_init(target, ZLIB_WINDOW_BITS, VERSION, STREAM_SIZE - 8).unwrap(),
            Z_VERSION_ERROR
        );
        assert_eq!(
            try_deflate_init(target, ZLIB_WINDOW_BITS, c"0.9", STREAM_SIZE).unwrap(),
            Z_VERSION_ERROR
        );
        assert_eq!(inflate_init(target, c"0.9", STREAM_SIZE), Z_VERSION_ERROR);
        assert_eq!(
            try_deflate_init(0, ZLIB_WINDOW_BITS, VERSION, STREAM_SIZE).unwrap(),
            Z_STREAM_ERROR
        );
        assert_eq!(inflate_init(0, VERSION, STREAM_SIZE), Z_STREAM_ERROR);
        assert_eq!((stream.zalloc, stream.zfree), (0, 0));
    }

    #[test]
    fn libcore_zip_calls_report_stream_and_memory_errors() {
        // cite: libcore Deflater.c#Z_STREAM_ERROR, Z_MEM_ERROR
        let mut idle = ZStream::default();
        assert_eq!(step("deflate", &mut idle, Z_FINISH), Z_STREAM_ERROR);
        assert_eq!(step("inflate", &mut idle, Z_NO_FLUSH), Z_STREAM_ERROR);
        assert_eq!(end("deflateEnd", &mut idle), Z_STREAM_ERROR);
        assert_eq!(end("inflateEnd", &mut idle), Z_STREAM_ERROR);

        let mut empty = [0usize; 2];
        let mut starved = ZStream {
            zalloc: allocators().block_alloc,
            zfree: allocators().block_free,
            opaque: empty.as_mut_ptr() as usize,
            ..ZStream::default()
        };
        assert_eq!(deflate_init(&mut starved, ZLIB_WINDOW_BITS), Z_MEM_ERROR);
    }

    #[test]
    fn a_failed_guest_allocator_call_fails_the_outer_zlib_call() {
        // cite: libcore zip_util.c#tracking_zlib_alloc
        register_all_groups();
        let mut stream = ZStream {
            zalloc: allocators().failing_alloc,
            zfree: allocators().block_free,
            ..ZStream::default()
        };
        let error = try_deflate_init(address(&mut stream), ZLIB_WINDOW_BITS, VERSION, STREAM_SIZE)
            .unwrap_err();
        assert!(error.to_string().contains("fx zlib allocator failed"));
        assert_eq!(stream.zalloc, allocators().failing_alloc);
    }
}
