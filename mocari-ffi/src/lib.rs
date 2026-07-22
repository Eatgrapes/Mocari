#![forbid(unsafe_op_in_unsafe_fn)]

use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};

use mocari::assets::RuntimeModel;

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("success").unwrap());
}

#[repr(C)]
pub struct MocariModelHandle {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MocariVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MocariMeshView {
    pub texture_index: i32,
    pub drawable_flags: u8,
    pub is_inverted_mask: u8,
    pub _reserved: [u8; 2],
    pub opacity: f32,
    pub draw_order: f32,
    pub render_order: i32,
    pub multiply_color: [f32; 3],
    pub screen_color: [f32; 3],
    pub vertices: *const MocariVertex,
    pub vertex_count: usize,
    pub indices: *const u16,
    pub index_count: usize,
    pub masks: *const i32,
    pub mask_count: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MocariTextureView {
    pub width: u32,
    pub height: u32,
    pub rgba: *const u8,
    pub byte_count: usize,
}

pub type MocariResult = i32;

pub const MOCARI_OK: MocariResult = 0;
pub const MOCARI_NULL_ARGUMENT: MocariResult = 1;
pub const MOCARI_INVALID_UTF8: MocariResult = 2;
pub const MOCARI_INVALID_HANDLE: MocariResult = 3;
pub const MOCARI_NOT_FOUND: MocariResult = 4;
pub const MOCARI_INVALID_OUTPUT: MocariResult = 5;
pub const MOCARI_RUNTIME_ERROR: MocariResult = 6;

struct FfiModel {
    model: RuntimeModel,
    vertices: Vec<Vec<MocariVertex>>,
}

impl FfiModel {
    fn new(model: RuntimeModel) -> Self {
        let vertices = model
            .runtime()
            .meshes()
            .iter()
            .map(|mesh| {
                mesh.vertices()
                    .iter()
                    .map(|vertex| MocariVertex {
                        position: vertex.position(),
                        uv: vertex.uv(),
                    })
                    .collect()
            })
            .collect();
        Self { model, vertices }
    }

    fn sync_vertices(&mut self) {
        for (storage, mesh) in self.vertices.iter_mut().zip(self.model.runtime().meshes()) {
            storage.clear();
            storage.extend(mesh.vertices().iter().map(|vertex| MocariVertex {
                position: vertex.position(),
                uv: vertex.uv(),
            }));
        }
    }
}

fn set_error(message: impl AsRef<str>) {
    let message =
        CString::new(message.as_ref()).unwrap_or_else(|_| CString::new("FFI error").unwrap());
    LAST_ERROR.with(|error| *error.borrow_mut() = message);
}

fn clear_error() {
    set_error("success");
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic in Mocari FFI")
        .to_owned()
}

unsafe fn model_mut<'a>(handle: *mut MocariModelHandle) -> Option<&'a mut FfiModel> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *handle.cast::<FfiModel>() })
    }
}

unsafe fn model_ref<'a>(handle: *const MocariModelHandle) -> Option<&'a FfiModel> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &*handle.cast::<FfiModel>() })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mocari_last_error_message() -> *const c_char {
    LAST_ERROR.with(|error| error.borrow().as_ptr().cast())
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `path` must point to a valid, NUL-terminated UTF-8 string for the duration
/// of this call.
pub unsafe extern "C" fn mocari_model_create(path: *const c_char) -> *mut MocariModelHandle {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let path = unsafe {
            if path.is_null() {
                return Err((MOCARI_NULL_ARGUMENT, "path is null".to_owned()));
            }
            CStr::from_ptr(path)
        };
        let path = path
            .to_str()
            .map_err(|_| (MOCARI_INVALID_UTF8, "path is not valid UTF-8".to_owned()))?;
        let model = mocari::assets::load_model_runtime(path)
            .map_err(|error| (MOCARI_RUNTIME_ERROR, error.to_string()))?;
        Ok(Box::into_raw(Box::new(FfiModel::new(model))).cast())
    }));

    match result {
        Ok(Ok(handle)) => {
            clear_error();
            handle
        }
        Ok(Err((_, message))) => {
            set_error(message);
            ptr::null_mut()
        }
        Err(payload) => {
            set_error(panic_message(payload));
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be null or a live handle returned by `mocari_model_create`.
/// Each non-null handle must be destroyed exactly once.
pub unsafe extern "C" fn mocari_model_destroy(handle: *mut MocariModelHandle) {
    if handle.is_null() {
        clear_error();
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(handle.cast::<FfiModel>()));
    }));
    match result {
        Ok(()) => clear_error(),
        Err(payload) => set_error(panic_message(payload)),
    }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle and `id` must point to a valid,
/// NUL-terminated UTF-8 string for the duration of this call.
pub unsafe extern "C" fn mocari_model_set_parameter(
    handle: *mut MocariModelHandle,
    id: *const c_char,
    value: f32,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_mut(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        if id.is_null() {
            return Err((MOCARI_NULL_ARGUMENT, "id is null"));
        }
        let id = CStr::from_ptr(id)
            .to_str()
            .map_err(|_| (MOCARI_INVALID_UTF8, "id is not valid UTF-8"))?;
        if model.model.runtime_mut().set_parameter(id, value) {
            Ok(())
        } else {
            Err((MOCARI_NOT_FOUND, "parameter was not found"))
        }
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle, `id` must point to a valid,
/// NUL-terminated UTF-8 string, and `value` must point to writable `f32`
/// storage for the duration of this call.
pub unsafe extern "C" fn mocari_model_get_parameter(
    handle: *const MocariModelHandle,
    id: *const c_char,
    value: *mut f32,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_ref(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        if id.is_null() {
            return Err((MOCARI_NULL_ARGUMENT, "id is null"));
        }
        if value.is_null() {
            return Err((MOCARI_INVALID_OUTPUT, "value is null"));
        }
        let id = CStr::from_ptr(id)
            .to_str()
            .map_err(|_| (MOCARI_INVALID_UTF8, "id is not valid UTF-8"))?;
        *value = model
            .model
            .runtime()
            .parameter_value(id)
            .ok_or((MOCARI_NOT_FOUND, "parameter was not found"))?;
        Ok(())
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle with no concurrent users.
pub unsafe extern "C" fn mocari_model_update(handle: *mut MocariModelHandle) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_mut(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        model
            .model
            .runtime_mut()
            .update_meshes()
            .ok_or((MOCARI_RUNTIME_ERROR, "mesh update failed"))?;
        model.sync_vertices();
        Ok(())
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle and `count` must point to writable
/// `usize` storage for the duration of this call.
pub unsafe extern "C" fn mocari_model_mesh_count(
    handle: *const MocariModelHandle,
    count: *mut usize,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_ref(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        let count = count
            .as_mut()
            .ok_or((MOCARI_INVALID_OUTPUT, "count is null"))?;
        *count = model.model.runtime().meshes().len();
        Ok(())
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle and `output` must point to writable
/// `MocariMeshView` storage for the duration of this call. Returned pointers
/// are borrowed from the handle and are invalidated by update or destroy.
pub unsafe extern "C" fn mocari_model_get_mesh(
    handle: *const MocariModelHandle,
    index: usize,
    output: *mut MocariMeshView,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_ref(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        let output = output
            .as_mut()
            .ok_or((MOCARI_INVALID_OUTPUT, "output is null"))?;
        let mesh = model
            .model
            .runtime()
            .meshes()
            .get(index)
            .ok_or((MOCARI_NOT_FOUND, "mesh was not found"))?;
        let vertices = model
            .vertices
            .get(index)
            .ok_or((MOCARI_RUNTIME_ERROR, "vertex storage is missing"))?;
        *output = MocariMeshView {
            texture_index: mesh.texture_index(),
            drawable_flags: mesh.drawable_flags(),
            is_inverted_mask: u8::from(mesh.is_inverted_mask()),
            _reserved: [0; 2],
            opacity: mesh.opacity(),
            draw_order: mesh.draw_order(),
            render_order: mesh.render_order(),
            multiply_color: mesh.multiply_color(),
            screen_color: mesh.screen_color(),
            vertices: vertices.as_ptr(),
            vertex_count: vertices.len(),
            indices: mesh.indices().as_ptr(),
            index_count: mesh.indices().len(),
            masks: mesh.masks().as_ptr(),
            mask_count: mesh.masks().len(),
        };
        Ok(())
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle and `count` must point to writable
/// `usize` storage for the duration of this call.
pub unsafe extern "C" fn mocari_model_texture_count(
    handle: *const MocariModelHandle,
    count: *mut usize,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_ref(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        let count = count
            .as_mut()
            .ok_or((MOCARI_INVALID_OUTPUT, "count is null"))?;
        *count = model.model.textures().len();
        Ok(())
    }));
    finish_unit(result)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `handle` must be a live model handle and `output` must point to writable
/// `MocariTextureView` storage for the duration of this call. Returned pixel
/// data is owned by Rust and remains valid until the handle is destroyed.
pub unsafe extern "C" fn mocari_model_get_texture(
    handle: *const MocariModelHandle,
    index: usize,
    output: *mut MocariTextureView,
) -> MocariResult {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe {
        let model = model_ref(handle).ok_or((MOCARI_INVALID_HANDLE, "handle is null"))?;
        let output = output
            .as_mut()
            .ok_or((MOCARI_INVALID_OUTPUT, "output is null"))?;
        let texture = model
            .model
            .textures()
            .get(index)
            .ok_or((MOCARI_NOT_FOUND, "texture was not found"))?;
        *output = MocariTextureView {
            width: texture.width(),
            height: texture.height(),
            rgba: texture.rgba().as_ptr(),
            byte_count: texture.rgba().len(),
        };
        Ok(())
    }));
    finish_unit(result)
}

fn finish_unit(
    result: std::thread::Result<Result<(), (MocariResult, &'static str)>>,
) -> MocariResult {
    match result {
        Ok(Ok(())) => {
            clear_error();
            MOCARI_OK
        }
        Ok(Err((code, message))) => {
            set_error(message);
            code
        }
        Err(payload) => {
            set_error(panic_message(payload));
            MOCARI_RUNTIME_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_path() -> CString {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("assets/models/Hiyori/Hiyori.model3.json");
        CString::new(path.to_string_lossy().as_bytes()).unwrap()
    }

    unsafe fn load_test_model() -> *mut MocariModelHandle {
        let path = model_path();
        let handle = unsafe { mocari_model_create(path.as_ptr()) };
        assert!(!handle.is_null(), "{}", unsafe {
            CStr::from_ptr(mocari_last_error_message()).to_string_lossy()
        });
        handle
    }

    #[test]
    fn null_arguments_return_errors() {
        assert_eq!(
            unsafe { mocari_model_update(ptr::null_mut()) },
            MOCARI_INVALID_HANDLE
        );
        unsafe { mocari_model_destroy(ptr::null_mut()) };
    }

    #[test]
    fn invalid_path_is_reported_without_panic() {
        let path = CString::new("missing.model3.json").unwrap();
        assert!(unsafe { mocari_model_create(path.as_ptr()) }.is_null());
        let message = unsafe { CStr::from_ptr(mocari_last_error_message()) };
        assert_ne!(message.to_bytes(), b"success");
    }

    #[test]
    fn invalid_utf8_and_null_outputs_are_rejected() {
        let invalid_path = [0xff_u8, 0];
        assert!(unsafe { mocari_model_create(invalid_path.as_ptr().cast()) }.is_null());
        let handle = unsafe { load_test_model() };

        assert_eq!(
            unsafe { mocari_model_get_parameter(handle, ptr::null(), ptr::null_mut()) },
            MOCARI_NULL_ARGUMENT
        );
        let id = CString::new("ParamAngleX").unwrap();
        assert_eq!(
            unsafe { mocari_model_get_parameter(handle, id.as_ptr(), ptr::null_mut()) },
            MOCARI_INVALID_OUTPUT
        );
        assert_eq!(
            unsafe { mocari_model_mesh_count(handle, ptr::null_mut()) },
            MOCARI_INVALID_OUTPUT
        );
        assert_eq!(
            unsafe { mocari_model_get_mesh(handle, 0, ptr::null_mut()) },
            MOCARI_INVALID_OUTPUT
        );
        assert_eq!(
            unsafe { mocari_model_texture_count(handle, ptr::null_mut()) },
            MOCARI_INVALID_OUTPUT
        );
        assert_eq!(
            unsafe { mocari_model_get_texture(handle, 0, ptr::null_mut()) },
            MOCARI_INVALID_OUTPUT
        );
        unsafe { mocari_model_destroy(handle) };
    }

    #[test]
    fn parameter_round_trip_updates_meshes_and_clears_error() {
        let handle = unsafe { load_test_model() };
        let missing = CString::new("MissingParameter").unwrap();
        assert_eq!(
            unsafe { mocari_model_set_parameter(handle, missing.as_ptr(), 1.0) },
            MOCARI_NOT_FOUND
        );

        let id = CString::new("ParamAngleX").unwrap();
        assert_eq!(
            unsafe { mocari_model_set_parameter(handle, id.as_ptr(), 12.5) },
            MOCARI_OK
        );
        let mut value = 0.0;
        assert_eq!(
            unsafe { mocari_model_get_parameter(handle, id.as_ptr(), &mut value) },
            MOCARI_OK
        );
        assert_eq!(value, 12.5);
        assert_eq!(unsafe { mocari_model_update(handle) }, MOCARI_OK);
        assert_eq!(
            unsafe { CStr::from_ptr(mocari_last_error_message()) }.to_bytes(),
            b"success"
        );
        unsafe { mocari_model_destroy(handle) };
    }

    #[test]
    fn multiple_handles_and_repeated_updates_remain_independent() {
        let first = unsafe { load_test_model() };
        let second = unsafe { load_test_model() };
        let id = CString::new("ParamAngleX").unwrap();

        assert_eq!(
            unsafe { mocari_model_set_parameter(first, id.as_ptr(), -12.5) },
            MOCARI_OK
        );
        assert_eq!(
            unsafe { mocari_model_set_parameter(second, id.as_ptr(), 12.5) },
            MOCARI_OK
        );

        for _ in 0..20 {
            assert_eq!(unsafe { mocari_model_update(first) }, MOCARI_OK);
            assert_eq!(unsafe { mocari_model_update(second) }, MOCARI_OK);
        }

        let mut first_value = 0.0;
        let mut second_value = 0.0;
        assert_eq!(
            unsafe { mocari_model_get_parameter(first, id.as_ptr(), &mut first_value) },
            MOCARI_OK
        );
        assert_eq!(
            unsafe { mocari_model_get_parameter(second, id.as_ptr(), &mut second_value) },
            MOCARI_OK
        );
        assert_eq!(first_value, -12.5);
        assert_eq!(second_value, 12.5);

        unsafe {
            mocari_model_destroy(first);
            mocari_model_destroy(second);
        }
    }

    #[test]
    fn out_of_range_views_are_rejected() {
        let handle = unsafe { load_test_model() };
        let mut mesh = std::mem::MaybeUninit::<MocariMeshView>::uninit();
        let mut texture = std::mem::MaybeUninit::<MocariTextureView>::uninit();
        assert_eq!(
            unsafe { mocari_model_get_mesh(handle, usize::MAX, mesh.as_mut_ptr()) },
            MOCARI_NOT_FOUND
        );
        assert_eq!(
            unsafe { mocari_model_get_texture(handle, usize::MAX, texture.as_mut_ptr()) },
            MOCARI_NOT_FOUND
        );
        unsafe { mocari_model_destroy(handle) };
    }

    #[test]
    fn real_model_can_be_loaded_and_exposed_as_mesh_views() {
        let handle = unsafe { load_test_model() };

        let mut count = 0;
        assert_eq!(
            unsafe { mocari_model_mesh_count(handle, &mut count) },
            MOCARI_OK
        );
        assert!(count > 0);

        let mut mesh = std::mem::MaybeUninit::<MocariMeshView>::uninit();
        assert_eq!(
            unsafe { mocari_model_get_mesh(handle, 0, mesh.as_mut_ptr()) },
            MOCARI_OK
        );
        let mesh = unsafe { mesh.assume_init() };
        assert!(mesh.vertex_count > 0);
        assert!(mesh.index_count > 0);
        assert!(!mesh.vertices.is_null());
        assert!(!mesh.indices.is_null());
        let vertices = unsafe { std::slice::from_raw_parts(mesh.vertices, mesh.vertex_count) };
        let indices = unsafe { std::slice::from_raw_parts(mesh.indices, mesh.index_count) };
        assert!(vertices.iter().all(|vertex| {
            vertex.position.iter().all(|value| value.is_finite())
                && vertex.uv.iter().all(|value| value.is_finite())
        }));
        assert!(
            indices
                .iter()
                .all(|index| usize::from(*index) < mesh.vertex_count)
        );

        let mut texture_count = 0;
        assert_eq!(
            unsafe { mocari_model_texture_count(handle, &mut texture_count) },
            MOCARI_OK
        );
        assert!(texture_count > 0);
        let mut texture = std::mem::MaybeUninit::<MocariTextureView>::uninit();
        assert_eq!(
            unsafe { mocari_model_get_texture(handle, 0, texture.as_mut_ptr()) },
            MOCARI_OK
        );
        let texture = unsafe { texture.assume_init() };
        assert!(texture.width > 0);
        assert!(texture.height > 0);
        assert_eq!(
            texture.byte_count,
            texture.width as usize * texture.height as usize * 4
        );
        assert!(!texture.rgba.is_null());
        unsafe { mocari_model_destroy(handle) };
    }

    #[test]
    fn ffi_layout_is_stable_on_64_bit_targets() {
        if usize::BITS == 64 {
            assert_eq!(std::mem::size_of::<MocariVertex>(), 16);
            assert_eq!(std::mem::align_of::<MocariVertex>(), 4);
            assert_eq!(std::mem::size_of::<MocariMeshView>(), 96);
            assert_eq!(std::mem::align_of::<MocariMeshView>(), 8);
            assert_eq!(std::mem::size_of::<MocariTextureView>(), 24);
            assert_eq!(std::mem::align_of::<MocariTextureView>(), 8);
            assert_eq!(std::mem::size_of::<MocariResult>(), 4);
        }
    }
}
